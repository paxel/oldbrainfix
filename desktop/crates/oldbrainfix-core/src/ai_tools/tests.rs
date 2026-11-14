use super::*;
use crate::store::tests::store;
use chrono::TimeZone;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// The tool `name` called with `args` for `task`, on a clock in UTC.
fn call(store: &Store, task: &str, name: &str, args: &Value) -> Result<ToolOutput> {
    store.ai_tool(task, name, args, &|t: DateTime<Utc>| t.naive_utc())
}

fn png(width: u32, height: u32) -> std::result::Result<Vec<u8>, image::ImageError> {
    let mut out = Cursor::new(Vec::new());
    image::RgbImage::from_pixel(width, height, image::Rgb([200, 40, 40]))
        .write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}

#[test]
fn a_photo_is_handed_out_scaled_down_and_only_from_its_note() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let receipt = notes.capture_with("Receipt", None, None, &[png(400, 200)?], &[])?;
    let other = notes.capture("Other", None, None)?;
    let task = store.open_ai_task()?;
    let hash = receipt.photos.first().ok_or("a photo")?;

    let out = call(
        &store,
        &task,
        "read_photo",
        &json!({"note": receipt.id, "photo": hash, "size": 100}),
    )?;
    let [ToolContent::Jpeg(jpeg)] = &out.content[..] else {
        return Err(format!("no JPEG: {out:?}").into());
    };
    let image = image::load_from_memory(jpeg)?;
    assert_eq!((image.width(), image.height()), (100, 50));

    let elsewhere = call(
        &store,
        &task,
        "read_photo",
        &json!({"note": other.id, "photo": hash}),
    )?;
    assert!(elsewhere.is_error);
    assert!(call(&store, &task, "send_note", &json!({})).is_err());
    Ok(())
}

#[test]
fn attached_files_are_read_with_their_texts_but_never_their_bytes() -> TestResult {
    use crate::notes::NewFile;
    let store = store();
    let notes = Notes::new(&store);
    let long = "Rent ".repeat(1000);
    let file = |name, bytes, pages| NewFile {
        name,
        media_type: "",
        bytes,
        pages,
    };
    let lease = notes.capture_with_files(
        "",
        None,
        None,
        &[],
        &[],
        &[
            file("Lease.pdf", b"%PDF-1.4 lease", Some(2)),
            file("Scan.pdf", b"%PDF-1.4 scan", Some(40)),
            file("plan.dwg", b"dwg", None),
        ],
    )?;
    let [pdf, scan, dwg] = [0, 1, 2].map(|i| lease.attachments[i].hash.clone());
    notes.set_attachment_text(&lease.id, &pdf, &long)?;
    let task = store.open_ai_task()?;
    let read = json_of(&call(&store, &task, "read_note", &json!({"id": lease.id}))?)?;
    let files = read["attachments"].as_array().ok_or("attachments")?;
    assert_eq!(files.len(), 3);
    assert_eq!(files[0]["name"], json!("Lease.pdf"));
    assert_eq!(files[0]["type"], json!("application/pdf"));
    assert_eq!(files[0]["size"], json!(14));
    assert_eq!(files[0]["pages"], json!(2));
    assert_eq!(files[0]["textState"], json!("read"));
    assert_eq!(files[0]["textLength"], json!(4999));
    assert_eq!(
        files[0]["text"].as_str().map(|t| t.chars().count()),
        Some(ATTACHMENT_TEXT_CHARS)
    );
    assert_eq!(files[1]["textState"], json!("fromComputer"));
    assert_eq!(files[2]["textState"], json!("none"));
    assert!(!read.to_string().contains("%PDF"), "no bytes go out");

    let rest = json_of(&call(
        &store,
        &task,
        "read_attachment_text",
        &json!({"note": lease.id, "file": pdf, "from": 4990, "size": 100}),
    )?)?;
    assert_eq!(
        rest,
        json!({"file": pdf, "from": 4990, "text": "Rent Rent", "total": 4999})
    );
    for hash in [&scan, &dwg] {
        let none = call(
            &store,
            &task,
            "read_attachment_text",
            &json!({"note": lease.id, "file": hash}),
        )?;
        assert!(none.is_error);
    }
    // No tool attaches a file or writes its text.
    assert!(tools().iter().all(|t| {
        let name = t.get("name").and_then(Value::as_str).unwrap_or("");
        name.starts_with("read") || !name.contains("attach")
    }));
    Ok(())
}

#[test]
fn an_area_is_deleted_with_its_topics_and_its_notes_go_back_to_unsorted() -> TestResult {
    let store = store();
    let places = Places::new(&store);
    let house = places.add_area("Household")?;
    let kitchen = places.add_topic(&house.id, "Kitchen")?;
    let milk = Notes::new(&store).capture("Milk", Some(&kitchen.id), None)?;
    let task = store.open_ai_task()?;

    let out = call(
        &store,
        &task,
        "delete_place",
        &json!({"id": house.id, "notesGo": "unsorted"}),
    )?;

    assert!(!out.is_error, "{out:?}");
    assert!(places.areas()?.is_empty());
    assert!(
        Notes::new(&store)
            .by_id(&milk.id)?
            .is_some_and(|n| n.is_unsorted())
    );
    assert_eq!(store.log_only(|| places.areas())?.len(), 1);
    let review = store.ai_review(&task)?;
    let gone: Vec<(String, String, Option<String>)> = review
        .groups
        .iter()
        .filter(|(kind, _)| *kind == crate::ai_task::AiChangeKind::Place)
        .flat_map(|(_, items)| items.iter())
        .flat_map(|i| {
            i.changes
                .iter()
                .map(|c| (i.entity.clone(), c.field.clone(), c.after.clone()))
        })
        .collect();
    assert_eq!(
        gone,
        [
            (kitchen.id.clone(), "$type".to_string(), None),
            (house.id.clone(), "$type".to_string(), None),
        ]
    );
    assert!(review.groups.iter().any(|(kind, items)| {
        *kind == crate::ai_task::AiChangeKind::NoteFiled && items[0].entity == milk.id
    }));
    Ok(())
}

#[test]
fn a_captured_detail_is_removed_as_a_pending_change_and_no_longer_read() -> TestResult {
    let store = store();
    let toner = Notes::new(&store).capture("Order toner", None, None)?.id;
    for (field, value) in [
        ("location", "52.52,13.405"),
        ("sourcePackage", "org.example.chat"),
        ("sourceLabel", "Chat"),
        ("calendar", r#"["Katzenklo"]"#),
    ] {
        store.append(&toner, field, Some(value), None)?;
    }
    let task = store.open_ai_task()?;
    let logged = store.all_entries()?.len();
    for detail in ["place", "app"] {
        let out = call(
            &store,
            &task,
            "remove_captured",
            &json!({"detail": detail, "id": toner}),
        )?;
        assert!(!out.is_error, "{out:?}");
    }
    let unknown = call(
        &store,
        &task,
        "remove_captured",
        &json!({"detail": "time", "id": toner}),
    )?;
    assert!(unknown.is_error);
    let again = call(
        &store,
        &task,
        "remove_captured",
        &json!({"detail": "place", "id": toner}),
    )?;
    assert!(again.is_error, "nothing left to remove");
    assert_eq!(store.all_entries()?.len(), logged);
    let read = call(&store, &task, "read_note", &json!({"id": toner}))?;
    let [ToolContent::Text(text)] = &read.content[..] else {
        return Err("a text".into());
    };
    let note: Value = serde_json::from_str(text)?;
    assert_eq!(
        note.get("captureContext"),
        Some(&json!({"calendar": r#"["Katzenklo"]"#}))
    );
    Ok(())
}

#[test]
fn the_text_format_is_read_and_set_as_a_pending_change() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let script = notes.capture("# comment\necho hi", None, None)?;
    let task = store.open_ai_task()?;
    let format_of = |id: &str| -> std::result::Result<Value, Box<dyn std::error::Error>> {
        let read = call(&store, &task, "read_note", &json!({"id": id}))?;
        let [ToolContent::Text(text)] = &read.content[..] else {
            return Err("a text".into());
        };
        let read: Value = serde_json::from_str(text)?;
        Ok(read["textFormat"].clone())
    };
    assert_eq!(format_of(&script.id)?, json!("plain"));
    let set = call(
        &store,
        &task,
        "set_text_format",
        &json!({"id": script.id, "format": "code"}),
    )?;
    assert!(!set.is_error, "{set:?}");
    assert_eq!(format_of(&script.id)?, json!("code"));
    let listed = call(&store, &task, "list_notes", &json!({}))?;
    assert!(format!("{listed:?}").contains(r#"\"textFormat\":\"code\""#));
    // A Code text becomes no Tally or Session.
    let fields = json!([{"name": "Run", "target": 5, "unit": "km"}]);
    let made = call(
        &store,
        &task,
        "make_tally",
        &json!({"fields": fields, "id": script.id}),
    )?;
    assert!(made.is_error, "{made:?}");
    let (pool, _) = tally(&store, "Push-ups")?;
    let session = call(
        &store,
        &task,
        "make_session",
        &json!({"note": script.id, "pool": [pool]}),
    )?;
    assert!(session.is_error, "{session:?}");
    // No such format, and no Note of another kind.
    let refused = call(
        &store,
        &task,
        "set_text_format",
        &json!({"id": script.id, "format": "x-future"}),
    )?;
    assert!(refused.is_error);
    let list = notes.capture("Trip\nSocks", None, None)?;
    Checklists::new(&store).make(&list.id)?;
    let refused = call(
        &store,
        &task,
        "set_text_format",
        &json!({"id": list.id, "format": "code"}),
    )?;
    assert!(refused.is_error);
    let review = store.ai_review(&task)?;
    assert!(review.groups.iter().any(|(kind, items)| {
        *kind == crate::ai_task::AiChangeKind::NoteText && items[0].entity == script.id
    }));
    Ok(())
}

#[test]
fn a_sections_text_is_edited_as_a_pending_change() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let note = notes.capture("Dentist", None, None)?;
    let amend = notes.amend(&note.id, "Moved to Wednesday")?;
    let later = notes.amend(&note.id, "Later")?;
    store.append(
        &later.id,
        crate::notes::keys::TEXT,
        Some(r##"{"format":"x-future","text":"# Later"}"##),
        None,
    )?;
    let task = store.open_ai_task()?;
    let logged = store.all_entries()?.len();
    let out = call(
        &store,
        &task,
        "edit_section",
        &json!({"id": amend.id, "text": "Moved to Thursday"}),
    )?;
    assert!(!out.is_error, "{out:?}");
    // A text in a format not known here, and no Section at all.
    for id in [&later.id, &note.id] {
        let refused = call(
            &store,
            &task,
            "edit_section",
            &json!({"id": id, "text": "Overwritten"}),
        )?;
        assert!(refused.is_error, "{id}");
    }
    assert_eq!(store.all_entries()?.len(), logged);
    let read = call(&store, &task, "read_note", &json!({"id": note.id}))?;
    let [ToolContent::Text(text)] = &read.content[..] else {
        return Err("a text".into());
    };
    let read: Value = serde_json::from_str(text)?;
    assert_eq!(
        read.pointer("/sections/0/text"),
        Some(&json!("Moved to Thursday"))
    );
    assert_eq!(read.pointer("/sections/0/mayEdit"), Some(&json!(true)));
    assert!(
        read.pointer("/sections/0/modified")
            .is_some_and(Value::is_string)
    );
    assert_eq!(read.pointer("/sections/1/mayEdit"), Some(&json!(false)));
    let review = store.ai_review(&task)?;
    assert!(review.groups.iter().any(|(kind, items)| {
        *kind == crate::ai_task::AiChangeKind::NoteText && items[0].entity == amend.id
    }));
    Ok(())
}

#[test]
fn a_change_waits_and_a_read_shows_it_as_the_user_sees_it() -> TestResult {
    let store = store();
    let milk = Notes::new(&store).capture("Milk", None, None)?;
    let task = store.open_ai_task()?;
    let logged = store.all_entries()?.len();
    let schedule =
        json!({"note": milk.id, "rule": "FREQ=DAILY", "start": "2026-01-02", "times": ["08:00"]});

    let added = call(&store, &task, "add_routine", &schedule)?;
    assert!(!added.is_error, "{added:?}");
    let read = call(&store, &task, "list_routines", &json!({"note": milk.id}))?;
    let [ToolContent::Text(text)] = &read.content[..] else {
        return Err("a text".into());
    };
    let routines: Value = serde_json::from_str(text)?;
    assert_eq!(routines.pointer("/0/times/0"), Some(&json!("08:00")));
    assert_eq!(routines.pointer("/0/rule"), Some(&json!("FREQ=DAILY")));
    assert_eq!(store.all_entries()?.len(), logged);
    DeviceSettings::new(&store).set_quick_note_shows(QuickNoteControl::FileInto, true)?;
    DeviceSettings::new(&store).set_agenda_hides_checked(true)?;
    DeviceSettings::new(&store).set_fat_fingers(true)?;
    PersonSettings::new(&store).set_trashes_done_reminder(true)?;
    let settings = call(&store, &task, "read_settings", &json!({}))?;
    assert!(!settings.is_error);
    let [ToolContent::Text(text)] = &settings.content[..] else {
        return Err("a text".into());
    };
    let read: Value = serde_json::from_str(text)?;
    assert_eq!(read.pointer("/quickNote/fileInto"), Some(&json!(true)));
    assert_eq!(read.pointer("/quickNote/formatBar"), Some(&json!(false)));
    assert_eq!(read.pointer("/quickNote/nudgeButtons"), Some(&json!(true)));
    assert_eq!(read.pointer("/theme"), Some(&json!("system")));
    assert_eq!(read.pointer("/aiReach"), Some(&json!("local")));
    assert_eq!(read.pointer("/allDayFrom"), Some(&json!("15:00")));
    assert_eq!(read.pointer("/agendaHidesChecked"), Some(&json!(true)));
    assert_eq!(read.pointer("/fatFingers"), Some(&json!(true)));
    assert_eq!(read.pointer("/trashesDoneReminder"), Some(&json!(true)));
    assert_eq!(read.pointer("/features"), Some(&json!("advanced")));
    PersonSettings::new(&store).set_advanced(false)?;
    let settings = call(&store, &task, "read_settings", &json!({}))?;
    assert_eq!(
        json_of(&settings)?.pointer("/features"),
        Some(&json!("simple"))
    );
    Ok(())
}

#[test]
fn the_areas_list_in_the_persons_own_order_and_a_drafted_one_last() -> TestResult {
    let store = store();
    let places = Places::new(&store);
    let work = places.add_area("Work")?.id;
    let home = places.add_area("Home")?.id;
    PersonSettings::new(&store).move_area(&work, 0)?;
    let task = store.open_ai_task()?;
    let added = call(&store, &task, "add_area", &json!({"name": "Attic"}))?;
    assert!(!added.is_error, "{added:?}");
    let listed = json_of(&call(&store, &task, "list_places", &json!({}))?)?;
    let names: Vec<_> = listed
        .as_array()
        .ok_or("a list")?
        .iter()
        .filter_map(|a| a.get("name").and_then(Value::as_str))
        .collect();
    assert_eq!(names, ["Work", "Home", "Attic"]);
    let ids: Vec<_> = PersonSettings::new(&store)
        .areas_in_order()?
        .into_iter()
        .map(|a| a.id)
        .take(2)
        .collect();
    assert_eq!(ids, [work, home]);
    Ok(())
}

#[test]
fn the_tags_list_by_their_levels_and_a_drafted_one_shows() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    notes.capture("#Life/health/back and #life/work", None, None)?;
    notes.capture("#Life/health", None, None)?;
    let gone = notes.capture("#gone", None, None)?;
    notes.trash(&gone.id)?;
    let task = store.open_ai_task()?;
    let added = call(&store, &task, "capture_note", &json!({"text": "#garten"}))?;
    assert!(!added.is_error, "{added:?}");
    let listed = json_of(&call(&store, &task, "list_tags", &json!({}))?)?;
    assert_eq!(
        listed,
        json!([
            {"children": [], "name": "garten", "notes": 1, "tag": "#garten"},
            {"children": [
                {"children": [
                    {"children": [], "name": "back", "notes": 1, "tag": "#Life/health/back"},
                ], "name": "health", "notes": 2, "tag": "#Life/health"},
                {"children": [], "name": "work", "notes": 1, "tag": "#Life/work"},
            ], "name": "Life", "notes": 2, "tag": "#Life"},
        ])
    );
    Ok(())
}

type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn utc(t: DateTime<Utc>) -> chrono::NaiveDateTime {
    t.naive_utc()
}

/// The text a tool handed back, read as JSON.
fn json_of(out: &ToolOutput) -> Made<Value> {
    let [ToolContent::Text(text)] = &out.content[..] else {
        return Err(format!("no text: {out:?}").into());
    };
    Ok(serde_json::from_str(text)?)
}

fn daily() -> Value {
    json!({"start": "2026-01-01", "times": ["08:00"], "rule": "FREQ=DAILY"})
}

fn schedule() -> Made<Schedule> {
    let eight: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let rule = Recurrence::parse("FREQ=DAILY").map_err(|e| format!("{e}"))?;
    Ok(Schedule::new(Day::new(2026, 1, 1), &eight, Some(rule)).ok_or("a schedule")?)
}

/// A Note made a Tally with one field: its id and the field's.
fn tally(store: &Store, text: &str) -> Made<(String, String)> {
    let note = Notes::new(store).capture(text, None, None)?.id;
    let field = Tallies::new(store).add_field(&note, text, "", None, None)?;
    Ok((note, field))
}

/// A Note made a Checklist of `text`.
fn checklist(store: &Store, text: &str) -> Made<String> {
    let note = Notes::new(store).capture(text, None, None)?.id;
    Checklists::new(store).make(&note)?;
    Ok(note)
}

fn shopping_list(store: &Store, text: &str) -> Made<String> {
    let note = Notes::new(store).capture(text, None, None)?.id;
    ShoppingLists::new(store).make(&note)?;
    Ok(note)
}

fn lines(store: &Store, note: &str) -> Made<Vec<crate::checklists::Line>> {
    Ok(Checklists::new(store)
        .by_note(note)?
        .ok_or("a Checklist")?
        .lines)
}

fn tally_field(store: &Store, note: &str) -> Made<Option<crate::tallies::TallyField>> {
    Ok(Tallies::new(store)
        .by_note(note)?
        .and_then(|t| t.fields.into_iter().next()))
}

/// Seeds a store, calls `name` with the args the seed gives and checks
/// with `shows` that the change waits, seen by the reads and not in
/// the log; that Accept keeps it; and, on another store, that Reject
/// leaves nothing.
fn lands_as_pending(
    name: &str,
    seed: impl Fn(&Store) -> Made<Value>,
    shows: impl Fn(&Store, &Value) -> Made<bool>,
) -> TestResult {
    for accept in [true, false] {
        let store = store();
        let args = seed(&store)?;
        let task = store.open_ai_task()?;
        let logged = store.all_entries()?.len();
        let out = call(&store, &task, name, &args)?;
        assert!(!out.is_error, "{name}: {out:?}");
        assert_eq!(store.all_entries()?.len(), logged, "{name} wrote the log");
        assert!(shows(&store, &args)?, "{name}: the reads show it");
        assert!(
            !store.log_only(|| shows(&store, &args))?,
            "{name}: the log holds it already"
        );
        assert!(!store.ai_review(&task)?.is_empty(), "{name}: reviewed");
        if accept {
            store.accept_pending(&task)?;
            assert!(shows(&store, &args)?, "{name}: kept on Accept");
        } else {
            store.decline_pending(&task)?;
            assert!(!shows(&store, &args)?, "{name}: gone on Reject");
            assert_eq!(store.all_entries()?.len(), logged, "{name}: nothing left");
        }
    }
    Ok(())
}

fn arg(args: &Value, key: &str) -> Made<String> {
    Ok(args
        .get(key)
        .and_then(Value::as_str)
        .ok_or("an id")?
        .to_string())
}

#[test]
fn every_tally_set_up_waits_for_the_review() -> TestResult {
    lands_as_pending(
        "make_tally",
        |s| {
            Ok(
                json!({"fields": [{"name": "Run", "target": 5, "unit": "km"}], "id": Notes::new(s).capture("Run", None, None)?.id}),
            )
        },
        |s, a| {
            let field = tally_field(s, &arg(a, "id")?)?;
            Ok(field.is_some_and(|f| f.unit == "km" && f.target == Some(5000.0)))
        },
    )?;
    lands_as_pending(
        "add_tally_field",
        |s| Ok(json!({"id": tally(s, "Run")?.0, "name": "Time", "rule": "max", "unit": "min"})),
        |s, a| {
            let tally = Tallies::new(s).by_note(&arg(a, "id")?)?.ok_or("a Tally")?;
            Ok(tally.fields.len() == 2 && tally.fields[1].rule == TallyRule::Max)
        },
    )?;
    lands_as_pending(
        "add_tally_field",
        |s| Ok(json!({"check": true, "id": tally(s, "Kitchen")?.0, "name": "Wiped"})),
        |s, a| {
            let tally = Tallies::new(s).by_note(&arg(a, "id")?)?.ok_or("a Tally")?;
            Ok(tally.fields.len() == 2 && tally.fields[1].check)
        },
    )?;
    lands_as_pending(
        "set_tally",
        |s| Ok(json!({"bar": "week", "id": tally(s, "Squats")?.0, "sets": true})),
        |s, a| {
            let tally = Tallies::new(s).by_note(&arg(a, "id")?)?.ok_or("a Tally")?;
            Ok(tally.bar == TallyPeriod::Week && tally.counts_in_sets)
        },
    )?;
    lands_as_pending(
        "rename_tally_field",
        |s| {
            let (id, field) = tally(s, "Pushups")?;
            Ok(json!({"field": field, "id": id, "name": "Push-ups"}))
        },
        |s, a| Ok(tally_field(s, &arg(a, "id")?)?.is_some_and(|f| f.name == "Push-ups")),
    )?;
    lands_as_pending(
        "set_tally_field",
        |s| {
            let (id, field) = tally(s, "Plank")?;
            Ok(json!({"field": field, "id": id, "rule": "max", "target": 2, "unit": "min"}))
        },
        |s, a| {
            let field = tally_field(s, &arg(a, "id")?)?.ok_or("a field")?;
            Ok(field.unit == "min" && field.target == Some(120.0) && field.rule == TallyRule::Max)
        },
    )?;
    lands_as_pending(
        "remove_tally_field",
        |s| {
            let (id, field) = tally(s, "Squats")?;
            Ok(json!({"field": field, "id": id}))
        },
        |s, a| Ok(tally_field(s, &arg(a, "id")?)?.is_some_and(|f| f.hidden)),
    )
}

#[test]
fn every_session_set_up_waits_for_the_review() -> TestResult {
    lands_as_pending(
        "make_session",
        |s| {
            let mut args = daily();
            let pool = vec![tally(s, "Push-ups")?.0, tally(s, "Squats")?.0];
            if let Some(all) = args.as_object_mut() {
                all.insert("count".into(), json!(1));
                all.insert("picking".into(), json!("random"));
                all.insert("pool".into(), json!(pool));
                all.insert("text".into(), json!("Workout"));
            }
            Ok(args)
        },
        |s, _| {
            let all = Sessions::new(s).all()?;
            Ok(all.len() == 1
                && all[0].setup.pool.len() == 2
                && all[0].setup.rule == SessionRule::Random)
        },
    )?;
    // Without a schedule the Note alone is set up.
    lands_as_pending(
        "make_session",
        |s| {
            let note = Notes::new(s).capture("Sport", None, None)?.id;
            Ok(json!({"note": note, "picking": "least", "pool": [tally(s, "Push-ups")?.0]}))
        },
        |s, a| {
            let note = arg(a, "note")?;
            let setup = Sessions::new(s).setup_of(&note)?;
            Ok(setup.is_some_and(|s| s.rule == SessionRule::Least)
                && Routines::new(s).of_note(&note)?.is_empty())
        },
    )?;
    lands_as_pending(
        "set_session",
        |s| {
            let (one, two) = (tally(s, "Push-ups")?.0, tally(s, "Squats")?.0);
            let session = Sessions::new(s).create(
                "Workout",
                &schedule()?,
                std::slice::from_ref(&one),
                1,
                SessionRule::Rolling,
            )?;
            Ok(
                json!({"count": 2, "id": session.note_id(), "picking": "least", "pool": [one, two], "weekdayCounts": {"6": 1}}),
            )
        },
        |s, a| {
            let session = Sessions::new(s)
                .setup_of(&arg(a, "id")?)?
                .ok_or("a Session")?;
            Ok(session.pool.len() == 2
                && session.count == 2
                && session.rule == SessionRule::Least
                && session.counts.get(&6) == Some(&1))
        },
    )
}

#[test]
fn every_list_set_up_waits_for_the_review() -> TestResult {
    for (list, is) in [("checklist", true), ("shoppingList", false)] {
        lands_as_pending(
            "make_list",
            |s| {
                Ok(
                    json!({"id": Notes::new(s).capture("Trip\nSocks\nShirt", None, None)?.id, "list": list}),
                )
            },
            |s, a| {
                let id = arg(a, "id")?;
                Ok(match is {
                    true => Checklists::new(s).is_checklist(&id)?,
                    false => ShoppingLists::new(s).is_shopping_list(&id)?,
                })
            },
        )?;
    }
    lands_as_pending(
        "list_to_plain",
        |s| Ok(json!({"id": checklist(s, "Trip\nSocks")?})),
        |s, a| Ok(!Checklists::new(s).is_checklist(&arg(a, "id")?)?),
    )?;
    lands_as_pending(
        "switch_list",
        |s| Ok(json!({"id": checklist(s, "Trip\nSocks")?})),
        |s, a| {
            ShoppingLists::new(s)
                .is_shopping_list(&arg(a, "id")?)
                .map_err(Into::into)
        },
    )?;
    lands_as_pending(
        "add_line",
        |s| Ok(json!({"at": 0, "id": checklist(s, "Trip\nSocks")?, "text": "Passport"})),
        |s, a| Ok(lines(s, &arg(a, "id")?)?[0].text == "Passport"),
    )?;
    lands_as_pending(
        "add_line",
        |s| Ok(json!({"id": shopping_list(s, "Shop\nMilk")?, "text": "Bread"})),
        |s, a| {
            let list = ShoppingLists::new(s).by_note(&arg(a, "id")?, &utc)?;
            Ok(list.is_some_and(|l| l.count() == 2))
        },
    )?;
    lands_as_pending(
        "rename_line",
        |s| Ok(json!({"id": lines(s, &checklist(s, "Trip\nSoks")?)?[0].id, "text": "Socks"})),
        |s, a| {
            let line = arg(a, "id")?;
            Ok(s.current(&line, checklist_keys::TEXT)?
                .is_some_and(|t| crate::StoredText::read(Some(&t)).text == "Socks"))
        },
    )?;
    lands_as_pending(
        "move_line",
        |s| {
            let note = checklist(s, "Trip\nSocks\nPassport")?;
            let all = lines(s, &note)?;
            Ok(json!({"before": all[0].id, "id": all[1].id, "note": note}))
        },
        |s, a| Ok(lines(s, &arg(a, "note")?)?[0].text == "Passport"),
    )?;
    lands_as_pending(
        "move_line",
        |s| {
            let note = shopping_list(s, "Shop\nMilk\nEggs")?;
            let list = ShoppingLists::new(s)
                .by_note(&note, &utc)?
                .ok_or("a list")?;
            Ok(json!({"id": list.lines[0].id, "note": note}))
        },
        |s, a| {
            let list = ShoppingLists::new(s)
                .by_note(&arg(a, "note")?, &utc)?
                .ok_or("a list")?;
            Ok(list.lines[0].text == "Eggs")
        },
    )?;
    lands_as_pending(
        "group_line",
        |s| {
            let note = checklist(s, "Trip\nBag\nSocks")?;
            Ok(json!({"id": lines(s, &note)?[1].id, "note": note, "under": true}))
        },
        |s, a| Ok(lines(s, &arg(a, "note")?)?[1].in_group()),
    )?;
    lands_as_pending(
        "set_reappear",
        |s| {
            let note = shopping_list(s, "Shop\nMilk")?;
            let list = ShoppingLists::new(s)
                .by_note(&note, &utc)?
                .ok_or("a list")?;
            Ok(
                json!({"count": 3, "id": list.lines[0].id, "note": note, "reappear": "after", "unit": "month"}),
            )
        },
        |s, a| {
            let list = ShoppingLists::new(s).by_note(&arg(a, "note")?, &utc)?;
            Ok(
                list.is_some_and(|l| {
                    l.lines[0].reappear == Reappear::After(3, ReappearUnit::Month)
                }),
            )
        },
    )?;
    lands_as_pending(
        "remove_line",
        |s| {
            let note = checklist(s, "Trip\nSocks\nShirt")?;
            Ok(json!({"id": lines(s, &note)?[0].id, "note": note}))
        },
        |s, a| Ok(lines(s, &arg(a, "note")?)?.len() == 1),
    )?;
    // Read from the log alone, the choice shows once accepted; the
    // Pending Change holds it until then.
    lands_as_pending(
        "set_shopping_agenda",
        |s| Ok(json!({"agenda": "lines", "id": shopping_list(s, "Shop\nMilk")?})),
        |s, a| {
            let entity = format!("{}{}", checklist_keys::AGENDA_PREFIX, arg(a, "id")?);
            Ok(s.current(&entity, shopping_keys::AGENDA)?.as_deref() == Some("lines"))
        },
    )?;
    lands_as_pending(
        "set_checklist_agenda",
        |s| Ok(json!({"agenda": "count", "id": checklist(s, "Trip\nSocks")?})),
        |s, a| {
            let entity = format!("{}{}", checklist_keys::AGENDA_PREFIX, arg(a, "id")?);
            Ok(s.current(&entity, checklist_keys::AGENDA)?.as_deref() == Some("count"))
        },
    )
}

#[test]
fn a_checklists_agenda_choice_is_set_once_accepted() -> TestResult {
    let store = store();
    let trip = checklist(&store, "Trip\nSocks")?;
    let shop = shopping_list(&store, "Shop\nMilk")?;
    let gym = checklist(&store, "Gym\nTowel")?;
    Routines::new(&store).spawn(
        &gym,
        &Schedule::new(Day::new(2026, 3, 1), &[], None).ok_or("a schedule")?,
    )?;
    let task = store.open_ai_task()?;
    let set = |id: &str, agenda: &str| {
        call(
            &store,
            &task,
            "set_checklist_agenda",
            &json!({"agenda": agenda, "id": id}),
        )
    };
    assert!(set(&shop, "off")?.is_error, "a Shopping List");
    assert!(set(&gym, "off")?.is_error, "a Checklist with a Routine");
    assert!(set(&trip, "sideways")?.is_error, "no such way");
    assert!(!set(&trip, "count")?.is_error);
    let review = store.ai_review(&task)?;
    let items: Vec<&crate::ai_task::AiReviewItem> =
        review.groups.iter().flat_map(|(_, i)| i).collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, crate::ai_task::AiChangeKind::List);
    assert_eq!(items[0].note.as_deref(), Some(trip.as_str()));
    let lists = Checklists::new(&store);
    assert_eq!(lists.on_agenda(&trip)?, ListOnAgenda::Lines);
    store.accept_pending(&task)?;
    assert_eq!(lists.on_agenda(&trip)?, ListOnAgenda::Count);
    let rows = lists.agenda_rows()?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].shows, ListOnAgenda::Count);
    Ok(())
}

#[test]
fn a_shopping_lists_agenda_choice_is_set_once_accepted() -> TestResult {
    let store = store();
    let shop = shopping_list(&store, "Shop\nMilk")?;
    let trip = checklist(&store, "Trip\nSocks")?;
    let task = store.open_ai_task()?;
    let set = |id: &str, agenda: &str| {
        call(
            &store,
            &task,
            "set_shopping_agenda",
            &json!({"agenda": agenda, "id": id}),
        )
    };
    assert!(set(&trip, "off")?.is_error, "a Checklist");
    assert!(set(&shop, "sideways")?.is_error, "no such way");
    assert!(!set(&shop, "off")?.is_error);
    let review = store.ai_review(&task)?;
    let items: Vec<&crate::ai_task::AiReviewItem> =
        review.groups.iter().flat_map(|(_, i)| i).collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, crate::ai_task::AiChangeKind::List);
    assert_eq!(items[0].note.as_deref(), Some(shop.as_str()));
    let lists = ShoppingLists::new(&store);
    assert_eq!(lists.on_agenda(&shop)?, ShoppingOnAgenda::Count);
    store.accept_pending(&task)?;
    assert_eq!(lists.on_agenda(&shop)?, ShoppingOnAgenda::Off);
    assert!(lists.agenda_rows(&utc)?.is_empty());
    Ok(())
}

#[test]
fn every_frog_set_up_waits_for_the_review() -> TestResult {
    lands_as_pending(
        "make_frog",
        |s| Ok(json!({"id": Notes::new(s).capture("Call the bank", None, None)?.id})),
        |s, a| Ok(!Frogs::new(s).of_note(&arg(a, "id")?)?.is_empty()),
    )?;
    lands_as_pending(
        "frog_to_plain",
        |s| {
            let note = Notes::new(s).capture("Call the bank", None, None)?.id;
            Frogs::new(s).spawn(&note)?;
            Ok(json!({"id": note}))
        },
        |s, a| Ok(Frogs::new(s).of_note(&arg(a, "id")?)?.is_empty()),
    )
}

#[test]
fn no_tool_writes_the_users_own_data() {
    let names = |read_only: bool| -> Vec<String> {
        tools()
            .iter()
            .filter(|t| t.pointer("/annotations/readOnlyHint") == Some(&Value::Bool(read_only)))
            .filter_map(|t| t.get("name").and_then(Value::as_str).map(str::to_string))
            .collect()
    };
    let writers = [
        "capture_note",
        "edit_note",
        "amend_note",
        "edit_section",
        "file_note",
        "merge_notes",
        "set_importance",
        "set_text_format",
        "remove_captured",
        "trash_note",
        "restore_note",
        "add_area",
        "add_topic",
        "rename_place",
        "set_area_colour",
        "set_area_icon",
        "delete_place",
        "add_routine",
        "reschedule_routine",
        "set_renag",
        "set_escalation",
        "trash_routine",
        "restore_routine",
        "make_tally",
        "set_tally",
        "add_tally_field",
        "rename_tally_field",
        "set_tally_field",
        "remove_tally_field",
        "make_session",
        "set_session",
        "make_list",
        "list_to_plain",
        "switch_list",
        "add_line",
        "rename_line",
        "move_line",
        "group_line",
        "set_reappear",
        "remove_line",
        "set_shopping_agenda",
        "set_checklist_agenda",
        "make_frog",
        "frog_to_plain",
    ];
    assert_eq!(
        names(false),
        writers,
        "only these write, as Pending Changes"
    );
    let readers = names(true);
    for data in [
        "read_tally_entries",
        "read_ticks",
        "read_check_offs",
        "read_skips",
        "read_eaten_frogs",
        "read_note_history",
        "read_session_picks",
    ] {
        assert!(readers.iter().any(|r| r == data), "{data} reads");
    }
    assert_eq!(readers.len() + writers.len(), tools().len());
}

#[test]
fn a_list_with_checked_lines_or_a_checked_name_is_refused_with_words() -> TestResult {
    let store = store();
    let trip = Notes::new(&store).capture("Trip\n✓ Socks\nShirt", None, None)?;
    let shop = shopping_list(&store, "Shop\nMilk")?;
    let milk = ShoppingLists::new(&store)
        .by_note(&shop, &utc)?
        .ok_or("a list")?;
    ShoppingLists::new(&store).check(&milk.lines[0].id, &utc)?;
    let task = store.open_ai_task()?;

    let made = call(
        &store,
        &task,
        "make_list",
        &json!({"id": trip.id, "list": "checklist"}),
    )?;
    assert!(made.is_error);
    assert!(format!("{made:?}").contains("✓"), "{made:?}");
    let added = call(
        &store,
        &task,
        "add_line",
        &json!({"id": shop, "text": " milk "}),
    )?;
    assert!(added.is_error, "{added:?}");
    let ticked = call(&store, &task, "move_line", &json!({"id": milk.lines[0].id}))?;
    assert!(ticked.is_error, "a struck Line does not move");
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
fn a_line_checked_once_today_reads_struck_and_gone_for_the_day() -> TestResult {
    let store = store();
    let shop = shopping_list(&store, "Shop\nMilk\nEggs")?;
    let lists = ShoppingLists::new(&store);
    let eggs = lists.by_note(&shop, &utc)?.ok_or("a list")?.lines[1]
        .id
        .clone();
    lists.set_reappear(&eggs, Reappear::Once)?;
    lists.check(&eggs, &utc)?;
    let task = store.open_ai_task()?;
    let lines = || -> Made<Vec<(Value, Value, Value)>> {
        let read = json_of(&call(&store, &task, "read_note", &json!({"id": shop}))?)?;
        Ok(read["lines"]
            .as_array()
            .ok_or("the Lines")?
            .iter()
            .map(|l| (l["text"].clone(), l["struck"].clone(), l["gone"].clone()))
            .collect())
    };
    assert_eq!(
        lines()?,
        [
            (json!("Milk"), json!(false), json!(false)),
            (json!("Eggs"), json!(true), json!(true)),
        ]
    );
    // Checked on a day before, it is gone from here too.
    store.append(
        &eggs,
        checklist_keys::CHECKED,
        Some("2025-12-31T10:00:00.000Z"),
        None,
    )?;
    assert_eq!(lines()?, [(json!("Milk"), json!(false), json!(false))]);
    Ok(())
}

#[test]
fn a_reappear_time_is_set_and_read_as_a_count_and_a_unit() -> TestResult {
    let store = store();
    let shop = shopping_list(&store, "Shop\nMilk")?;
    let milk = ShoppingLists::new(&store)
        .by_note(&shop, &|t: DateTime<Utc>| t.naive_utc())?
        .ok_or("a list")?
        .lines[0]
        .id
        .clone();
    let task = store.open_ai_task()?;
    let set = |args: Value| call(&store, &task, "set_reappear", &args);
    let line = || -> Made<Value> {
        let out = call(&store, &task, "read_note", &json!({"id": shop}))?;
        let [ToolContent::Text(text)] = &out.content[..] else {
            return Err("no text".into());
        };
        let read: Value = serde_json::from_str(text)?;
        Ok(read
            .pointer("/lines/0/reappear")
            .cloned()
            .unwrap_or_default())
    };
    assert_eq!(line()?, json!({"kind": "none"}));
    for (args, read) in [
        (
            json!({"count": 20, "unit": "week"}),
            json!({"count": 20, "kind": "after", "unit": "week"}),
        ),
        (
            json!({"count": 1, "unit": "year"}),
            json!({"count": 1, "kind": "after", "unit": "year"}),
        ),
        (
            json!({"count": 99, "unit": "day"}),
            json!({"count": 99, "kind": "after", "unit": "day"}),
        ),
    ] {
        let mut args = args;
        args["id"] = json!(milk);
        args["reappear"] = json!("after");
        assert!(!set(args)?.is_error);
        assert_eq!(line()?, read);
    }
    assert!(!set(json!({"id": milk, "reappear": "once"}))?.is_error);
    assert_eq!(line()?, json!({"kind": "once"}));
    for refused in [
        json!({"id": milk, "reappear": "after", "count": 0, "unit": "day"}),
        json!({"id": milk, "reappear": "after", "count": 100, "unit": "day"}),
        json!({"id": milk, "reappear": "after", "count": 3}),
        json!({"id": milk, "reappear": "after", "unit": "month"}),
        json!({"id": milk, "reappear": "after", "count": 3, "unit": "fortnight"}),
        json!({"id": milk, "reappear": "3m"}),
    ] {
        let out = set(refused.clone());
        assert!(out.is_err() || out.is_ok_and(|o| o.is_error), "{refused}");
    }
    assert_eq!(line()?, json!({"kind": "once"}));
    assert!(!set(json!({"id": milk, "reappear": "none"}))?.is_error);
    assert_eq!(line()?, json!({"kind": "none"}));
    Ok(())
}

#[test]
fn a_note_from_a_pack_names_its_sender_while_it_keeps_the_badge() -> TestResult {
    use crate::{OwnProfile, SigningKey};
    let person = |seed: u8, name: &str| -> Made<Store> {
        let key = move || Ok(SigningKey::from_seed([seed; 32]));
        let store = Store::in_memory(crate::store::tests::ticking(), key)?;
        OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    };
    let ann = person(1, "Ann")?;
    let ben = person(2, "Ben")?;
    let note = Notes::new(&ann).capture("Drink water", None, None)?;
    let pack = ann.pack_of(&note.id, &std::collections::HashSet::new())?;
    let file = ann.write_pack(&pack, &pack.all_keys(), true)?;
    let opened = ben.open_pack(&file)?.map_err(|e| e.name())?;
    let wall = |t: DateTime<Utc>| t.naive_utc();
    let ticks = opened.pack.all_keys();
    ben.add_pack(&opened, &ticks, false, &wall, "Pack", "From a Pack")?;
    let badged = Notes::new(&ben).all()?.remove(0).id;
    ben.add_pack(&opened, &ticks, true, &wall, "Pack", "From a Pack")?;
    let claimed = Notes::new(&ben).all()?.remove(0).id;
    assert_ne!(badged, claimed);
    let task = ben.open_ai_task()?;
    let read = |id: &str| json_of(&call(&ben, &task, "read_note", &json!({"id": id}))?);
    assert_eq!(read(&badged)?.get("from"), Some(&json!("Ann")));
    assert_eq!(read(&claimed)?.get("from"), None);
    Ok(())
}

#[test]
fn notes_name_their_kind_and_parts() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let plain = notes.capture("Plain ☐ box", None, None)?.id;
    let (run, field) = tally(&store, "Run")?;
    Tallies::new(&store).set_unit(&run, &field, "km")?;
    Tallies::new(&store).set_target(&run, &field, Some(5000.0))?;
    let trip = checklist(&store, "Trip\nBag\n Socks")?;
    let shop = shopping_list(&store, "Shop\nMilk")?;
    let bank = notes.capture("Call the bank", None, None)?.id;
    let frog = Frogs::new(&store).spawn(&bank)?;
    let workout = Sessions::new(&store).spawn(
        &bank,
        &schedule()?,
        std::slice::from_ref(&run),
        1,
        SessionRule::Least,
    )?;
    let task = store.open_ai_task()?;

    let listed = json_of(&call(&store, &task, "list_notes", &json!({}))?)?;
    let kind = |id: &str| -> Option<Value> {
        listed
            .as_array()?
            .iter()
            .find(|n| n.get("id") == Some(&json!(id)))
            .cloned()
    };
    let kind_of = |id: &str| kind(id).and_then(|n| n.get("kind").cloned());
    assert_eq!(kind_of(&plain), Some(json!("plain")));
    assert_eq!(kind_of(&run), Some(json!("tally")));
    assert_eq!(kind_of(&trip), Some(json!("checklist")));
    assert_eq!(kind_of(&shop), Some(json!("shoppingList")));
    assert_eq!(kind_of(&bank), Some(json!("session")));
    let bank_listed = kind(&bank).ok_or("the bank's Note")?;
    assert_eq!(bank_listed.get("frog"), Some(&json!(frog.id)));
    assert_eq!(bank_listed.get("sessions"), Some(&json!([workout.id()])));

    let read = |id: &str| json_of(&call(&store, &task, "read_note", &json!({"id": id}))?);
    let tallied = read(&run)?;
    assert_eq!(tallied.pointer("/tally/fields/0/unit"), Some(&json!("km")));
    assert_eq!(tallied.pointer("/tally/fields/0/target"), Some(&json!(5.0)));
    let packed = read(&trip)?;
    assert_eq!(packed.pointer("/lines/1/text"), Some(&json!("Socks")));
    assert!(
        packed
            .pointer("/lines/1/group")
            .is_some_and(|g| g.is_string())
    );
    assert_eq!(
        read(&shop)?.pointer("/lines/0/reappear"),
        Some(&json!({"kind": "none"}))
    );
    assert_eq!(read(&shop)?.pointer("/agenda"), Some(&json!("count")));
    assert_eq!(read(&trip)?.pointer("/name"), Some(&json!("Trip")));
    assert_eq!(read(&trip)?.pointer("/description"), Some(&json!("")));
    assert_eq!(read(&plain)?.pointer("/name"), Some(&Value::Null));
    let gym = tally(&store, "Gym\nMondays and Thursdays")?.0;
    let gymmed = read(&gym)?;
    assert_eq!(gymmed.pointer("/name"), Some(&json!("Gym")));
    assert_eq!(
        gymmed.pointer("/description"),
        Some(&json!("Mondays and Thursdays"))
    );
    assert_eq!(read(&trip)?.pointer("/agenda"), Some(&json!("lines")));
    assert_eq!(read(&plain)?.pointer("/agenda"), Some(&Value::Null));
    let banked = read(&bank)?;
    assert_eq!(banked.pointer("/frogs/0/id"), Some(&json!(frog.id)));
    assert_eq!(
        banked.pointer("/routines/0/session/picking"),
        Some(&json!("least"))
    );
    assert_eq!(
        banked.pointer("/routines/0/session/pool"),
        Some(&json!([run]))
    );
    assert_eq!(banked.pointer("/session/pool"), Some(&json!([run])));
    assert_eq!(banked.pointer("/session/picking"), Some(&json!("least")));
    assert_eq!(
        banked.pointer("/session/routines"),
        Some(&json!([workout.id()]))
    );
    assert_eq!(read(&trip)?.pointer("/session"), Some(&Value::Null));
    assert_eq!(
        read(&plain)?.pointer("/boxes/0/state"),
        Some(&json!("open"))
    );
    Ok(())
}

#[test]
fn an_entry_reads_with_its_sets_and_what_each_field_counts() -> TestResult {
    let store = store();
    let (note, reps) = tally(&store, "Push-ups")?;
    let tallies = Tallies::new(&store);
    let done = tallies.add_check(&note, "Stretched", None)?;
    tallies.set_counts_in_sets(&note, true)?;
    tallies.add_with(
        &note,
        &BTreeMap::from([(reps.clone(), 10.0), (done.clone(), 1.0)]),
        &[BTreeMap::from([(reps.clone(), 8.0)])],
        "",
        None,
        |_| Ok(()),
    )?;
    let task = store.open_ai_task()?;
    let read = |name: &str, args: Value| json_of(&call(&store, &task, name, &args)?);
    let entries = read("read_tally_entries", json!({"note": note}))?;
    assert_eq!(entries.pointer("/0/values/0/field"), Some(&json!(reps)));
    assert_eq!(entries.pointer("/0/values/0/value"), Some(&json!(18.0)));
    assert_eq!(entries.pointer("/0/values/1/value"), Some(&json!(1.0)));
    assert_eq!(
        entries.pointer("/0/sets"),
        Some(&json!([{reps.as_str(): 10.0}, {reps.as_str(): 8.0}]))
    );
    let shown = read("read_note", json!({"id": note}))?;
    assert_eq!(shown.pointer("/tally/sets"), Some(&json!(true)));
    assert_eq!(shown.pointer("/tally/bar"), Some(&json!("day")));
    assert_eq!(shown.pointer("/tally/fields/1/check"), Some(&json!(true)));
    Ok(())
}

#[test]
fn the_users_own_data_is_read_with_its_dates_and_by_range() -> TestResult {
    let store = store();
    let at = |day: u32| {
        Utc.with_ymd_and_hms(2025, 10, day, 18, 0, 0)
            .single()
            .ok_or("a time")
    };
    let (run, field) = tally(&store, "Run")?;
    let tallies = Tallies::new(&store);
    for day in [3, 17] {
        tallies.add(
            &run,
            &BTreeMap::from([(field.clone(), 4.0)]),
            "park",
            Some(at(day)?),
        )?;
    }
    let trip = checklist(&store, "Trip\nSocks\nShirt")?;
    Checklists::new(&store).check(&lines(&store, &trip)?[0].id)?;
    let bank = Notes::new(&store).capture("Call the bank", None, None)?.id;
    let frog = Frogs::new(&store).spawn(&bank)?;
    Frogs::new(&store).mark_done(&frog.id, Some(at(20)?))?;
    let pills = Notes::new(&store).capture("Pills", None, None)?.id;
    let routine = Routines::new(&store).spawn(&pills, &schedule()?)?;
    let first = Occurrence::new(Day::new(2026, 1, 1), ClockTime::new(8, 0));
    Routines::new(&store).mark_done(&routine.id, &first, None)?;
    let session = Sessions::new(&store).spawn(
        &bank,
        &schedule()?,
        std::slice::from_ref(&run),
        1,
        SessionRule::Rolling,
    )?;
    Sessions::new(&store).pick_today(&utc)?;
    Sessions::new(&store).record(session.id(), &first, &run, &BTreeMap::new(), "done")?;
    let task = store.open_ai_task()?;
    let read = |name: &str, args: Value| json_of(&call(&store, &task, name, &args)?);

    let entries = read("read_tally_entries", json!({"note": run}))?;
    assert_eq!(
        entries.as_array().map(Vec::len),
        Some(3),
        "with the Session's"
    );
    let october = read(
        "read_tally_entries",
        json!({"from": "2025-10-01", "note": run, "until": "2025-10-31"}),
    )?;
    assert_eq!(october.pointer("/0/day"), Some(&json!("2025-10-17")));
    assert_eq!(october.pointer("/1/at"), Some(&json!(dart_iso(at(3)?))));
    assert_eq!(october.pointer("/1/values/0/value"), Some(&json!(4.0)));
    assert_eq!(october.as_array().map(Vec::len), Some(2));
    let ticks = read("read_ticks", json!({"note": trip}))?;
    assert_eq!(ticks.pointer("/0/text"), Some(&json!("Socks")));
    assert_eq!(ticks.pointer("/0/day"), Some(&json!("2026-01-01")));
    let none = read("read_ticks", json!({"note": trip, "until": "2025-12-31"}))?;
    assert_eq!(none, json!([]));
    let offs = read("read_check_offs", json!({"note": pills}))?;
    assert_eq!(offs.pointer("/0/occurrence"), Some(&json!(first.key())));
    assert!(offs.pointer("/0/at").is_some_and(Value::is_string));
    let second = Occurrence::new(Day::new(2026, 1, 2), ClockTime::new(8, 0));
    let third = Occurrence::new(Day::new(2026, 1, 3), ClockTime::new(8, 0));
    Routines::new(&store).skip(&routine.id, &second, Some(at(21)?))?;
    Routines::new(&store).tomorrow(&routine.id, &third, Some(at(22)?))?;
    let skips = read("read_skips", json!({"routine": routine.id}))?;
    assert_eq!(skips.pointer("/0/occurrence"), Some(&json!(third.key())));
    assert_eq!(skips.pointer("/0/at"), Some(&json!(dart_iso(at(22)?))));
    // A daily Routine has tomorrow's own: none added.
    assert_eq!(skips.pointer("/0/tomorrow"), Some(&Value::Null));
    assert_eq!(skips.pointer("/1/occurrence"), Some(&json!(second.key())));
    assert_eq!(skips.as_array().map(Vec::len), Some(2));
    let none = read(
        "read_skips",
        json!({"routine": routine.id, "from": "2026-01-04"}),
    )?;
    assert_eq!(none, json!([]));
    let eaten = read("read_eaten_frogs", json!({"from": "2025-10-20"}))?;
    assert_eq!(eaten.pointer("/0/at"), Some(&json!(dart_iso(at(20)?))));
    assert_eq!(
        read("read_eaten_frogs", json!({"from": "2025-10-21"}))?,
        json!([])
    );
    let picks = read("read_session_picks", json!({"id": session.id()}))?;
    assert_eq!(picks.pointer("/0/day"), Some(&json!("2026-01-01")));
    assert_eq!(picks.pointer("/0/picks/0/note"), Some(&json!(run)));
    assert!(
        picks
            .pointer("/0/picks/0/entries/0/at")
            .is_some_and(Value::is_string)
    );
    let changes = read("read_note_history", json!({"note": trip}))?;
    assert_eq!(changes.pointer("/0/change"), Some(&json!("lineChecked")));
    assert_eq!(changes.pointer("/0/after"), Some(&json!("Socks")));
    assert_eq!(changes.pointer("/0/day"), Some(&json!("2026-01-01")));
    let last = changes
        .as_array()
        .and_then(|c| c.last())
        .ok_or("a change")?;
    assert_eq!(last.get("change"), Some(&json!("captured")));
    assert_eq!(
        read(
            "read_note_history",
            json!({"note": trip, "until": "2025-12-31"})
        )?,
        json!([])
    );
    assert!(
        call(
            &store,
            &task,
            "read_note_history",
            &json!({"note": "note:none"})
        )?
        .is_error
    );
    let bad = call(
        &store,
        &task,
        "read_eaten_frogs",
        &json!({"from": "October"}),
    )?;
    assert!(bad.is_error);
    Ok(())
}

#[test]
fn find_notes_ranks_the_notes_holding_most_words_first_with_an_excerpt() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let one = notes.capture("Insurance: the bike is covered", None, None)?;
    let both = notes.capture(
        "Bike insurance letter\nThe policy number is 77-3",
        None,
        None,
    )?;
    notes.capture("Groceries", None, None)?;
    let gone = notes.capture("Bike trip insurance", None, None)?;
    notes.trash(&gone.id)?;
    let out = json_of(&call(
        &store,
        "t",
        "find_notes",
        &json!({"words": ["insurance", "policy", "  "]}),
    )?)?;
    assert_eq!(out["found"], 2);
    assert_eq!(out["notes"][0]["id"], json!(both.id));
    assert_eq!(out["notes"][0]["title"], "Bike insurance letter");
    assert_eq!(out["notes"][0]["words"], json!(["insurance", "policy"]));
    assert_eq!(out["notes"][1]["id"], json!(one.id));
    assert!(
        out["notes"][0]["excerpt"]
            .as_str()
            .is_some_and(|e| e.contains("policy number is 77-3"))
    );
    assert!(out["notes"][0].get("text").is_none());
    let limited = json_of(&call(
        &store,
        "t",
        "find_notes",
        &json!({"words": ["insurance"], "limit": 1}),
    )?)?;
    assert_eq!(limited["notes"].as_array().map(Vec::len), Some(1));
    assert_eq!(limited["found"], 2);
    assert!(call(&store, "t", "find_notes", &json!({"words": []}))?.is_error);
    Ok(())
}

#[test]
fn list_new_lists_the_notes_captured_or_amended_since_newest_first() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let old = notes.capture("Old Note", None, None)?;
    let older = notes.capture("Older Note", None, None)?;
    let fresh = notes.capture("Fresh Note", None, None)?;
    let since = dart_iso(fresh.captured);
    notes.amend(&old.id, "A new line")?;
    let out = json_of(&call(&store, "t", "list_new", &json!({"since": since}))?)?;
    assert_eq!(out["found"], 2);
    assert_eq!(out["notes"][0]["id"], json!(old.id));
    assert!(out["notes"][0]["amended"].is_string());
    assert_eq!(out["notes"][1]["id"], json!(fresh.id));
    assert_eq!(out["notes"][1]["unsorted"], true);
    assert!(out["notes"][1]["amended"].is_null());
    let today = json_of(&call(
        &store,
        "t",
        "list_new",
        &json!({"since": "2026-01-01"}),
    )?)?;
    assert_eq!(today["found"], 3);
    let later = json_of(&call(
        &store,
        "t",
        "list_new",
        &json!({"since": "2026-01-02"}),
    )?)?;
    assert_eq!(later["found"], 0);
    assert!(call(&store, "t", "list_new", &json!({"since": "soon"}))?.is_error);
    let _ = older;
    Ok(())
}

#[test]
fn the_asking_tools_leave_out_what_is_hidden_from_ai() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let area = Places::new(&store).add_area("Diary")?;
    let shown = notes.capture("Tax return 2026", None, None)?;
    let hidden = notes.capture("Tax debts, private", None, None)?;
    let filed = notes.capture("Tax adviser's letter", Some(&area.id), None)?;
    notes.amend(&shown.id, "tax receipts")?;
    store.set_hidden_from_ai(&hidden.id, true)?;
    store.set_hidden_from_ai(&area.id, true)?;
    let found = call(&store, "t", "find_notes", &json!({"words": ["tax"]}))?;
    let new = call(&store, "t", "list_new", &json!({"since": "2026-01-01"}))?;
    for out in [&found, &new] {
        assert_eq!(json_of(out)?["found"], 1);
        let text = format!("{out:?}");
        assert!(!text.contains(&hidden.id) && !text.contains(&filed.id));
        assert!(!text.contains("private") && !text.contains("adviser"));
    }
    Ok(())
}

#[test]
fn the_ask_panel_reads_without_an_ai_task_and_drafts_its_first_change_into_one() -> TestResult {
    let store = store();
    let note = Notes::new(&store).capture("Dentist", None, None)?;
    let area = Places::new(&store).add_area("Health")?;
    let mut task = None;
    let read = store.ask_tool("read_note", &json!({"id": note.id}), &utc, &mut task)?;
    assert!(!read.is_error);
    assert_eq!((task.clone(), store.ai_task()?), (None, None));
    assert!(!store.sync_paused()?);
    // What the panel does not offer, it refuses.
    for unoffered in ["list_notes", "read_photo", "trash_note", "no_such"] {
        let out = store.ask_tool(unoffered, &json!({}), &utc, &mut task)?;
        assert!(out.is_error, "{unoffered}");
    }
    assert_eq!(task, None);
    let filed = store.ask_tool(
        "file_note",
        &json!({"id": note.id, "place": area.id}),
        &utc,
        &mut task,
    )?;
    assert!(!filed.is_error, "{filed:?}");
    assert_eq!(task, store.ai_task()?);
    assert!(task.is_some() && store.sync_paused()?);
    Ok(())
}
