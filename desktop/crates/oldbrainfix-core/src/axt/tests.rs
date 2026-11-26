use super::*;
use crate::store::tests::ticking;
use crate::{OwnProfile, SigningKey, text_format};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn utc(at: DateTime<Utc>) -> chrono::NaiveDateTime {
    at.naive_utc()
}

/// A Device of its own person named `name`.
fn device(seed: u8, name: &str) -> Made<Store> {
    let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    value.field(name).unwrap_or(&Value::Null)
}

struct Made4 {
    plain: String,
    formatted: String,
    dentist: String,
    pills: String,
}

fn notes(ann: &Store) -> Made<Made4> {
    let notes = Notes::new(ann);
    let routines = Routines::new(ann);
    let plain = notes
        .capture("Shopping\nMilk and #Eggs ☐ #eggs", None, None)?
        .id;
    let formatted = notes
        .capture_as(
            "# Garden\nWater the **roses** #garden/roses",
            text_format::MARKDOWN,
            None,
            None,
        )?
        .id;
    let dentist = notes.capture("Dentist\nAt the new place", None, None)?.id;
    let ten = ClockTime::new(10, 0).ok_or("10:00")?;
    let reminder = routines.spawn(
        &dentist,
        &Schedule::new(Day::new(2026, 12, 3), &[ten], None).ok_or("once")?,
    )?;
    routines.mark_done(
        &reminder.id,
        &Occurrence::new(Day::new(2026, 12, 3), Some(ten)),
        None,
    )?;
    let pills = notes.capture("Pills", None, None)?.id;
    routines.spawn(
        &pills,
        &Schedule::new(
            Day::new(2026, 11, 1),
            &[
                ClockTime::new(8, 0).ok_or("8")?,
                ClockTime::new(20, 0).ok_or("20")?,
            ],
            Some(Recurrence::parse("FREQ=DAILY;COUNT=10").map_err(|e| e.message)?),
        )
        .ok_or("daily")?,
    )?;
    Ok(Made4 {
        plain,
        formatted,
        dentist,
        pills,
    })
}

fn datetime(fields: [i32; 5], zoned: Option<(&str, i32, i64)>) -> Value {
    let [year, month, day, hour, minute] = fields;
    let (zone, offset, instant) = match zoned {
        Some((z, o, i)) => (s(z), Value::Int(o), Value::Long(i)),
        None => (Value::Null, Value::Null, Value::Null),
    };
    record(vec![
        ("year", Value::Int(year)),
        ("month", Value::Int(month)),
        ("day", Value::Int(day)),
        ("hour", Value::Int(hour)),
        ("minute", Value::Int(minute)),
        ("second", Value::Int(0)),
        ("millisecond", Value::Int(0)),
        ("zone", zone),
        ("offset_seconds", offset),
        ("instant_ms", instant),
    ])
}

fn duration(days: i32, millis: i64) -> Value {
    record(vec![
        ("months", Value::Int(0)),
        ("days", Value::Int(days)),
        ("millis", Value::Long(millis)),
    ])
}

/// A file of another app, as the Dart test's: an Appointment at 10:00
/// in Berlin's winter time with two reminders before it and one after, a
/// Task done on its due day, and a Memo, a type Old Brain Fix does not
/// take.
fn catalog_file() -> Made<Vec<u8>> {
    let descriptions = type_descriptions()?;
    let memo = TypeDescription::from_json(&json!({
        "name": "memo",
        "doc": "A memo.",
        "texts": {},
        "kind": {"axt.ObjectField": {
            "name": "org.example.Memo",
            "version": 1,
            "fields": [{
                "name": "text",
                "doc": "The text.",
                "texts": {},
                "required": true,
                "kind": {"axt.StringField": {}},
            }],
        }},
    }))
    .map_err(|e| e.to_string())?;
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = 200 - i as u8;
    }
    let mut config = NodeConfig::new(record(vec![
        ("app_name", s("cat-a-log")),
        ("app_url", Value::Null),
        ("formats", Value::Array(vec![Value::Int(1)])),
        ("takes", Value::Array(Vec::new())),
        ("any_type", Value::Null),
        ("sends", Value::Null),
        ("encryption_key", Value::Null),
        ("relay", Value::Null),
    ]));
    config.clock = Box::new(|| UNIX_EPOCH + StdDuration::from_secs(1_795_000_000));
    let mut node = AxtNode::open(
        MemoryStore::default(),
        SigningSeed::new(seed),
        EncryptionSeed::new([3; 32]),
        config,
    );
    let instant = DateTime::parse_from_rfc3339("2026-12-03T09:00:00Z")?.timestamp_millis();
    let appointment = record(vec![
        ("title", s("Vet: Mia")),
        (
            "start",
            datetime([2026, 12, 3, 10, 0], Some(("Europe/Berlin", 3600, instant))),
        ),
        ("end", Value::Null),
        ("all_day", Value::Boolean(false)),
        ("location", s("Main Street 4")),
        ("notes", s("Bring the booklet")),
        (
            "reminders",
            Value::Array(vec![
                duration(-1, 0),
                duration(0, -1_200_000),
                duration(0, 600_000),
            ]),
        ),
        ("relations", Value::Array(Vec::new())),
        ("extensions", Value::Array(Vec::new())),
    ]);
    let task = record(vec![
        ("title", s("Order food")),
        ("notes", s("")),
        ("due", datetime([2026, 11, 18, 0, 0], None)),
        ("due_all_day", Value::Boolean(true)),
        ("repeat", s("FREQ=MONTHLY")),
        ("done", Value::Boolean(true)),
        ("done_at", Value::Null),
        (
            "priority",
            Value::Enum {
                index: 0,
                symbol: "NONE".into(),
            },
        ),
        ("relations", Value::Array(Vec::new())),
        ("extensions", Value::Array(Vec::new())),
    ]);
    let [_, task_type, appointment_type] = descriptions.as_slice() else {
        return Err("three types".into());
    };
    Ok(node
        .write_file(
            &[
                OutgoingRecord::new(
                    "0b9a8f1e-3c2d-4e5f-8a7b-6c5d4e3f2a10",
                    appointment_type,
                    appointment,
                ),
                OutgoingRecord::new("1c2d3e4f-5a6b-4c7d-8e9f-a0b1c2d3e4f5", task_type, task),
                OutgoingRecord::new(
                    "2d3e4f5a-6b7c-4d8e-9fa0-b1c2d3e4f5a6",
                    &memo,
                    record(vec![("text", s("Not for Old Brain Fix"))]),
                ),
            ],
            None,
            false,
        )
        .map_err(|e| e.to_string())?)
}

fn read(store: &Store, bytes: &[u8]) -> Made<AxtFile> {
    Ok(store.axt_read(bytes, &utc)?.map_err(|_| "no AXT file")?)
}

#[test]
fn a_note_goes_as_a_note_one_with_a_reminder_or_routine_as_a_task() -> TestResult {
    let ann = device(1, "Ann")?;
    let n = notes(&ann)?;
    let (plain_type, plain) = ann.axt_record_of(&n.plain)?.ok_or("plain")?;
    assert_eq!(plain_type, types::NOTE);
    assert_eq!(field(&plain, "title"), &s("Shopping"));
    assert_eq!(
        field(&plain, "text"),
        &s("Shopping\nMilk and #Eggs ☐ #eggs")
    );
    assert_eq!(field(&plain, "tags"), &Value::Array(vec![s("Eggs")]));
    let (_, formatted) = ann.axt_record_of(&n.formatted)?.ok_or("formatted")?;
    assert_eq!(field(&formatted, "title"), &s("Garden"));
    assert!(
        matches!(field(&formatted, "format"), Value::Enum { symbol, .. } if symbol == "MARKDOWN")
    );
    assert_eq!(
        field(&formatted, "tags"),
        &Value::Array(vec![s("garden/roses")])
    );
    let (dentist_type, dentist) = ann.axt_record_of(&n.dentist)?.ok_or("dentist")?;
    assert_eq!(dentist_type, types::TASK);
    assert_eq!(field(&dentist, "notes"), &s("At the new place"));
    assert_eq!(field(&dentist, "repeat"), &Value::Null);
    assert_eq!(field(&dentist, "done"), &Value::Boolean(true));
    assert_eq!(field(field(&dentist, "due"), "hour"), &Value::Int(10));
    let (_, pills) = ann.axt_record_of(&n.pills)?.ok_or("pills")?;
    assert_eq!(
        field(&pills, "repeat"),
        &s("FREQ=DAILY;COUNT=10;BYHOUR=8,20;BYMINUTE=0")
    );
    Ok(())
}

#[test]
fn the_record_id_is_the_note_id_as_a_uuid_of_version_4_one_of_its_own_for_each_type() {
    let note = "note:000000000000000000000000000000ab";
    assert_eq!(
        record_id(note, types::NOTE),
        "00000000-0000-4000-8000-0000000000ab"
    );
    let task = record_id(note, types::TASK);
    assert_ne!(task, record_id(note, types::NOTE));
    assert_eq!(task.chars().nth(14), Some('4'));
    let other = record_id("note:help-start", types::NOTE);
    assert_eq!(other.len(), 36);
    assert_eq!(other.chars().nth(14), Some('4'));
    assert_eq!(record_id("note:help-start", types::NOTE), other);
}

#[test]
fn a_file_of_one_device_comes_back_as_the_notes_it_was_written_from() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let n = notes(&ann)?;
    let ids = [&n.plain, &n.formatted, &n.dentist, &n.pills, "note:gone"]
        .map(str::to_string)
        .to_vec();
    let bytes = ann.axt_write(&ids)?;
    let file = read(&ben, &bytes)?;
    assert_eq!(file.sender, hex::encode(ann.signing_key()?.public_key()));
    assert_eq!(file.app_name.as_deref(), Some("Old Brain Fix"));
    assert!(!file.from_own_device);
    assert!(file.not_taken().is_empty());
    let read_ids: Vec<_> = file.records.iter().map(|r| r.id.clone()).collect();
    let want = vec![
        record_id(&n.plain, types::NOTE),
        record_id(&n.formatted, types::NOTE),
        record_id(&n.dentist, types::TASK),
        record_id(&n.pills, types::TASK),
    ];
    assert_eq!(read_ids, want);
    let drafts: Vec<_> = file
        .records
        .iter()
        .filter_map(|r| r.draft.clone())
        .collect();
    assert_eq!(drafts[0].text, "Shopping\nMilk and #Eggs ☐ #eggs");
    assert_eq!(drafts[0].format, text_format::PLAIN);
    assert_eq!(
        drafts[1].text,
        "# Garden\nWater the **roses** #garden/roses"
    );
    assert_eq!(drafts[1].format, text_format::MARKDOWN);
    assert_eq!(drafts[2].text, "Dentist\nAt the new place");
    let dentist = drafts[2].schedule.as_ref().ok_or("a Reminder")?;
    assert_eq!(dentist.start, Day::new(2026, 12, 3));
    assert!(drafts[2].done);
    let pills = drafts[3].schedule.as_ref().ok_or("a Routine")?;
    assert_eq!(pills.day_times().len(), 2);
    assert!(read(&ann, &bytes)?.from_own_device);
    Ok(())
}

#[test]
fn reading_takes_nothing_and_keeps_nothing() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let n = notes(&ann)?;
    let bytes = ann.axt_write(std::slice::from_ref(&n.plain))?;
    let entries = ben.all_entries()?.len();
    let settings = ben.local_settings_with_prefix("")?.len();
    let first = read(&ben, &bytes)?;
    let second = read(&ben, &bytes)?;
    assert_eq!(ben.all_entries()?.len(), entries);
    assert_eq!(ben.local_settings_with_prefix("")?.len(), settings);
    assert_eq!(first.records, second.records);
    Ok(())
}

#[test]
fn a_file_changed_after_it_was_signed_is_no_axt_file() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let n = notes(&ann)?;
    let bytes = ann.axt_write(std::slice::from_ref(&n.plain))?;
    for at in [bytes.len() / 2, bytes.len() - 20] {
        let mut changed = bytes.clone();
        changed[at] ^= 0x01;
        assert_eq!(ben.axt_read(&changed, &utc)?, Err(NotAnAxtFile));
    }
    assert_eq!(ben.axt_read(b"{}", &utc)?, Err(NotAnAxtFile));
    Ok(())
}

#[test]
fn a_second_file_of_the_same_note_is_a_later_version() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let n = notes(&ann)?;
    let ids = std::slice::from_ref(&n.plain);
    let first = read(&ben, &ann.axt_write(ids)?)?;
    Notes::new(&ann).edit(&n.plain, "Shopping\nMilk")?;
    let second = read(&ben, &ann.axt_write(ids)?)?;
    assert_eq!(second.records[0].id, first.records[0].id);
    assert!(second.records[0].version > first.records[0].version);
    Ok(())
}

#[test]
fn another_apps_file_an_appointment_a_task_done_and_a_type_not_taken() -> TestResult {
    let ben = device(2, "Ben")?;
    let file = read(&ben, &catalog_file()?)?;
    assert_eq!(file.app_name.as_deref(), Some("cat-a-log"));
    let taken = file.taken();
    let vet = taken[0].draft.as_ref().ok_or("vet")?;
    assert_eq!(vet.text, "Vet: Mia\nMain Street 4\nBring the booklet");
    let at = vet.schedule.as_ref().ok_or("a Reminder")?;
    assert_eq!(at.times, vec![ClockTime::new(9, 0).ok_or("9")?]);
    assert_eq!(vet.heads_ups, vec![20, 1440]);
    let food = taken[1].draft.as_ref().ok_or("food")?;
    let monthly = food.schedule.as_ref().ok_or("a Routine")?;
    assert!(monthly.times.is_empty());
    assert_eq!(
        monthly.rule.as_ref().map(|r| r.format()).as_deref(),
        Some("FREQ=MONTHLY")
    );
    assert!(food.done);
    let memo = file.not_taken();
    assert_eq!(memo.len(), 1);
    assert_eq!(memo[0].type_name.as_deref(), Some("org.example.Memo"));
    assert_eq!(memo[0].result, "APPLIED");
    Ok(())
}

#[test]
fn the_chosen_records_become_notes_with_where_they_came_from() -> TestResult {
    let ben = device(2, "Ben")?;
    let file = read(&ben, &catalog_file()?)?;
    let vet = Places::new(&ben).add_area("Vet")?;
    let taken = file.taken();
    let (vet_record, food) = (taken[0].clone(), taken[1].clone());
    let made = ben.axt_take(&file, std::slice::from_ref(&vet_record.id), Some(&vet.id))?;
    let note = Notes::new(&ben).by_id(&made[0])?.ok_or("made")?;
    assert_eq!(note.place.as_deref(), Some(vet.id.as_str()));
    let routine = Routines::new(&ben)
        .of_note(&note.id)?
        .pop()
        .ok_or("a Reminder")?;
    assert_eq!(Routines::new(&ben).heads_ups(&routine.id)?, vec![20, 1440]);
    let from = ben.axt_from(&note.id)?.ok_or("from")?;
    assert_eq!(from.sender, file.sender);
    assert_eq!(from.id, vet_record.id);
    assert_eq!(from.version, vet_record.version);
    assert_eq!(from.type_name, types::APPOINTMENT);
    let unsorted = ben.axt_take(&file, std::slice::from_ref(&food.id), None)?;
    let task = Routines::new(&ben)
        .of_note(&unsorted[0])?
        .pop()
        .ok_or("a Routine")?;
    assert!(
        task.done_at(&Occurrence::new(Day::new(2026, 11, 18), None))
            .is_some()
    );
    let memo = file.not_taken()[0].id.clone();
    assert!(ben.axt_take(&file, &[memo], None)?.is_empty());
    let before = ben.all_entries()?.len();
    assert!(
        ben.axt_take(&file, &[vet_record.id], Some("area:x"))
            .is_err()
    );
    assert_eq!(ben.all_entries()?.len(), before);
    Ok(())
}

#[test]
fn what_another_persons_device_wrote_counts_for_nothing() -> TestResult {
    let ben = device(2, "Ben")?;
    let file = read(&ben, &catalog_file()?)?;
    let first = file.taken()[0].id.clone();
    let taken = ben.axt_take(&file, &[first], None)?.remove(0);
    let own = Notes::new(&ben).capture("Milk", None, None)?.id;
    let kept = ben.axt_from(&taken)?;
    // As a modified app in a Shared Area would: Cy's Device writes the
    // field of Ben's Notes.
    let cy = device(3, "Cy")?;
    let forged = AxtFrom {
        sender: "ab".repeat(32),
        id: "3e4f5a6b-7c8d-4e9f-a0b1-c2d3e4f5a6b7".into(),
        type_name: types::NOTE.into(),
        version: 1 << 60,
    }
    .encode();
    for id in [&taken, &own] {
        ben.insert(&cy.append(id, keys::FROM, Some(&forged), None)?.wire())?;
    }
    assert_eq!(ben.axt_from(&taken)?, kept);
    assert_eq!(ben.axt_from(&own)?, None);
    Ok(())
}

/// Another app that keeps what it took between files, as one does: one
/// library instance on one store, taking Notes and Tasks; it answers what
/// it made of each record of a file, by id.
struct Receiver {
    node: AxtNode<MemoryStore>,
    held: BTreeMap<String, Option<Value>>,
}

impl Receiver {
    fn open() -> Made<Receiver> {
        let take = |t: &str| {
            record(vec![
                ("type", s(t)),
                ("min_version", Value::Int(1)),
                ("max_version", Value::Null),
                ("purpose", s("It becomes a page.")),
                ("purpose_translations", Value::Map(BTreeMap::new())),
            ])
        };
        let mut config = NodeConfig::new(record(vec![
            ("app_name", s("second-brain")),
            ("app_url", Value::Null),
            ("formats", Value::Array(vec![Value::Int(1)])),
            (
                "takes",
                Value::Array(vec![take(types::NOTE), take(types::TASK)]),
            ),
            ("any_type", Value::Null),
            ("sends", Value::Null),
            ("encryption_key", Value::Null),
            ("relay", Value::Null),
        ]));
        config.readers = type_descriptions()?.into_iter().take(2).collect();
        Ok(Receiver {
            node: AxtNode::open(
                MemoryStore::default(),
                SigningSeed::new([77; 32]),
                EncryptionSeed::new([3; 32]),
                config,
            ),
            held: BTreeMap::new(),
        })
    }

    fn take(&mut self, bytes: &[u8]) -> Made<BTreeMap<String, String>> {
        let preview = self.node.inspect(bytes).map_err(|e| e.to_string())?;
        let held = &mut self.held;
        let mut taken = Vec::new();
        let current = held.clone();
        let result = self
            .node
            .apply(
                &preview,
                true,
                |_, change| {
                    taken.push((change.id.to_lowercase(), change.value));
                    Ok(())
                },
                |_, _, id| Ok(current.get(id).cloned().flatten()),
            )
            .map_err(|e| e.to_string())?;
        held.extend(taken);
        Ok(result
            .records
            .into_iter()
            .map(|r| (r.id, r.result.as_str().to_string()))
            .collect())
    }
}

#[test]
fn another_app_that_keeps_what_it_took_takes_every_later_file_as_an_update() -> TestResult {
    let ann = device(1, "Ann")?;
    let n = notes(&ann)?;
    let mut receiver = Receiver::open()?;
    let send = |receiver: &mut Receiver, note: &str, type_name: &str| -> Made<Option<String>> {
        let result = receiver.take(&ann.axt_write(&[note.to_string()])?)?;
        Ok(result.get(&record_id(note, type_name)).cloned())
    };
    let updated = |r: Option<String>| matches!(r.as_deref(), Some("APPLIED" | "MERGED"));
    let all = [&n.plain, &n.formatted, &n.dentist, &n.pills].map(|s| s.to_string());
    let results = receiver.take(&ann.axt_write(&all)?)?;
    assert!(results.values().all(|r| r == "APPLIED"), "{results:?}");
    // Its text edited, a Box ticked, an Occurrence Done.
    Notes::new(&ann).edit(&n.plain, "Shopping\nMilk and #Eggs ☐ #eggs and bread")?;
    assert!(updated(send(&mut receiver, &n.plain, types::NOTE)?));
    let note = Notes::new(&ann).by_id(&n.plain)?.ok_or("plain")?;
    let box_id = note.boxes.first().ok_or("a Box")?.id.clone();
    crate::Boxes::new(&ann).tick(&n.plain, &box_id, crate::BoxState::Done)?;
    assert!(updated(send(&mut receiver, &n.plain, types::NOTE)?));
    let pills = Routines::new(&ann)
        .of_note(&n.pills)?
        .pop()
        .ok_or("pills")?;
    Routines::new(&ann).mark_done(
        &pills.id,
        &Occurrence::new(Day::new(2026, 11, 1), ClockTime::new(8, 0)),
        None,
    )?;
    assert!(updated(send(&mut receiver, &n.pills, types::TASK)?));
    // It gains a Reminder: a Task of its own; losing it, the Note again.
    let reminder = Routines::new(&ann).spawn(
        &n.plain,
        &Schedule::new(
            Day::new(2026, 12, 1),
            &[ClockTime::new(9, 0).ok_or("9")?],
            None,
        )
        .ok_or("once")?,
    )?;
    assert_eq!(
        send(&mut receiver, &n.plain, types::TASK)?.as_deref(),
        Some("APPLIED")
    );
    Routines::new(&ann).trash(&reminder.id)?;
    assert!(updated(send(&mut receiver, &n.plain, types::NOTE)?));
    // Sent beside a Note changed later, then alone.
    Notes::new(&ann).edit(&n.formatted, "# Garden\nWater the roses")?;
    receiver.take(&ann.axt_write(&[n.plain.clone(), n.formatted.clone()])?)?;
    assert!(updated(send(&mut receiver, &n.plain, types::NOTE)?));
    Ok(())
}

/// One clock for every Device of the sharing test, a second further at
/// each reading, so what was written later wins everywhere.
static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

fn sharing_device(seed: u8, name: &str) -> Made<Store> {
    let start = ticking()();
    let clock: crate::Clock = Box::new(move || {
        let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        start + chrono::Duration::seconds(n)
    });
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

fn remember(store: &Store, peer: &Store) -> TestResult {
    store.remember_device(&crate::PairedDevice {
        id: peer.device_id()?,
        name: String::new(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: store.now(),
        shared: true,
    })?;
    Ok(())
}

/// Ann's Household shared with Ben as a Member.
fn household() -> Made<(Store, Store, String)> {
    use crate::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
    use crate::shared::{AreaRole, Invite};
    let ann = sharing_device(1, "Ann")?;
    let ben = sharing_device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    let invite = Invite {
        area: area.clone(),
        role: AreaRole::Member,
    };
    let request = sync_request_as(
        &ben,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    remember(&ann, &ben)?;
    remember(&ben, &ann)?;
    Ok((ann, ben, area))
}

#[test]
fn a_member_takes_no_reminder_into_a_shared_area_and_one_that_left_takes_nothing_there()
-> TestResult {
    let (ann, ben, area) = household()?;
    let file = read(&ben, &catalog_file()?)?;
    let before = ben.all_entries()?.len();
    let vet = file.taken()[0].id.clone();
    assert!(ben.axt_take(&file, &[vet], Some(&area)).is_err());
    assert_eq!(ben.all_entries()?.len(), before);
    let milk = Notes::new(&ann).capture("Milk", None, None)?.id;
    let plain = read(&ben, &ann.axt_write(&[milk])?)?;
    let id = plain.taken()[0].id.clone();
    let made = ben.axt_take(&plain, std::slice::from_ref(&id), Some(&area))?;
    let note = Notes::new(&ben).by_id(&made[0])?.ok_or("made")?;
    assert_eq!(note.place.as_deref(), Some(area.as_str()));
    ben.leave_area(&area)?;
    let after = ben.all_entries()?.len();
    assert!(ben.axt_take(&plain, &[id], Some(&area)).is_err());
    assert_eq!(ben.all_entries()?.len(), after);
    Ok(())
}
