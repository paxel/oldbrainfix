use super::*;
use crate::lan::{
    Joining, answer_sync, answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
};
use crate::shared::{AreaRole, Invite};
use crate::signing::SigningKey;
use crate::store::tests::ticking;
use crate::{Entry, OwnProfile, Places, Search};
use chrono::{Duration, NaiveDateTime, TimeZone};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One clock for every Device of a test, a second further at each
/// reading, so what was written later wins everywhere.
static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

fn device(seed: u8, name: &str) -> Made<Store> {
    let start = ticking()();
    let clock: crate::Clock = Box::new(move || {
        let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        start + Duration::seconds(n)
    });
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

/// The wall clock on UTC.
fn utc_wall(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.naive_utc()
}

fn at(y: i32, m: u32, d: u32, h: u32) -> Made<DateTime<Utc>> {
    Ok(Utc
        .with_ymd_and_hms(y, m, d, h, 0, 0)
        .single()
        .ok_or("a time")?)
}

fn everyone() -> PersonFilter {
    PersonFilter::default()
}

fn values<S: AsRef<str>>(pairs: &[(S, f64)]) -> BTreeMap<String, f64> {
    pairs
        .iter()
        .map(|(k, v)| (k.as_ref().to_string(), *v))
        .collect()
}

fn bars(chart: &[TallyBucket]) -> Vec<Option<f64>> {
    chart.iter().map(|b| b.value).collect()
}

/// A Tally made from a new Note with `text` and a field of each of
/// `units`, named after its unit.
fn tally(
    store: &Store,
    text: &str,
    units: &[&str],
    place: Option<&str>,
) -> Made<(String, Vec<String>)> {
    let note = Notes::new(store).capture(text, place, None)?;
    let mut fields = Vec::new();
    for unit in units {
        fields.push(Tallies::new(store).add_field(&note.id, unit, unit, None, None)?);
    }
    Ok((note.id, fields))
}

/// One LAN session, `joiner` asking `host`, between two people.
fn exchange(joiner: &Store, host: &Store) -> TestResult {
    let request = sync_request(
        joiner,
        &host.version_vector()?,
        "Phone",
        &host.device_id()?,
        "",
    )?;
    let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
    apply_answer_as(joiner, &answer, true, None)?;
    Ok(())
}

/// Ann's Household shared with Ben as a Member.
fn household() -> Made<(Store, Store, String)> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
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
    for (store, peer) in [(&ann, &ben), (&ben, &ann)] {
        store.remember_device(&crate::PairedDevice {
            id: peer.device_id()?,
            name: String::new(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: store.now(),
            shared: true,
        })?;
    }
    Ok((ann, ben, area))
}

#[test]
fn a_first_field_makes_a_note_a_tally_and_takes_it_out_of_unsorted() -> TestResult {
    let ann = device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let tallies = Tallies::new(&ann);
    let note = notes.capture("Push-ups", None, None)?;
    assert_eq!(tallies.by_note(&note.id)?, None);
    assert_eq!(notes.unsorted()?.len(), 1);
    let reps = tallies.add_field(&note.id, " Push-ups ", "times", Some(20.0), None)?;
    let weight = tallies.add_field(&note.id, "Weight", "kg", None, None)?;
    assert!(notes.unsorted()?.is_empty());
    let made = tallies.by_note(&note.id)?.ok_or("a Tally")?;
    let ids: Vec<&str> = made.fields.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, [reps.as_str(), weight.as_str()]);
    assert_eq!(made.fields[0].name, "Push-ups");
    assert_eq!(made.fields[0].rule, TallyRule::Sum);
    assert_eq!(made.fields[1].rule, TallyRule::Max);
    assert_eq!(made.targets(), values(&[(&reps, 20.0)]));
    assert!(tallies.add_field(&note.id, " ", "", None, None).is_err());
    Ok(())
}

#[test]
fn fields_saved_together_a_microsecond_apart_keep_their_order() -> TestResult {
    // As the New Tally page saves them (#161): in one batch, each a
    // read of the clock after the one before.
    let start = ticking()();
    let ticks = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(0));
    let clock: crate::Clock = Box::new(move || {
        let n = ticks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        start + Duration::microseconds(n)
    });
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([3; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    let note = Notes::new(&store).capture("Morning", None, None)?;
    let tallies = Tallies::new(&store);
    let names = ["Weight", "Waist", "Steps", "Arm", "Calf", "Back"];
    store.batch(|| {
        for name in names {
            tallies.add_field(&note.id, name, "", None, None)?;
        }
        Ok(())
    })?;
    let made = tallies.by_note(&note.id)?.ok_or("a Tally")?;
    let shown: Vec<&str> = made.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(shown, names);
    Ok(())
}

#[test]
fn a_fields_kind_of_unit_is_fixed_once_it_has_entries() -> TestResult {
    let ann = device(1, "Ann")?;
    let tallies = Tallies::new(&ann);
    let (note, fields) = tally(&ann, "Lifting", &["kg", "reps"], None)?;
    let (weight, other) = (&fields[0], &fields[1]);
    tallies.set_unit(&note, other, "min")?;
    tallies.set_unit(&note, other, "reps")?;
    let entry = tallies.add(&note, &values(&[(weight, 80.0)]), "", None)?;
    tallies.set_unit(&note, weight, "g")?;
    assert!(tallies.set_unit(&note, weight, "km").is_err());
    assert!(tallies.set_unit(&note, weight, "bags").is_err());
    tallies.trash_entry(&entry.id)?;
    assert!(tallies.set_unit(&note, weight, "km").is_err());
    tallies.restore_entry(&entry.id)?;
    assert_eq!(
        tallies.entries(&note, &everyone())?[0].values.get(weight),
        Some(&80.0)
    );
    Ok(())
}

#[test]
fn a_tally_and_its_fields_are_read_without_its_entries_once_per_change() -> TestResult {
    use std::sync::atomic::Ordering::SeqCst;
    let ann = device(1, "Ann")?;
    let tallies = Tallies::new(&ann);
    let (note, fields) = tally(&ann, "Push-ups", &["times"], None)?;
    let reps = &fields[0];
    for _ in 0..20 {
        tallies.add(&note, &values(&[(reps, 1.0)]), "", None)?;
    }
    let plain = Notes::new(&ann).capture("Plain", None, None)?.id;
    // Worked out once: the Note and its fields, no entry.
    let before = ann.history_reads.load(SeqCst);
    assert!(tallies.is_tally(&note)?);
    assert!(!tallies.is_tally(&plain)?);
    assert_eq!(ann.history_reads.load(SeqCst) - before, 4);
    // Then kept until the log changes.
    let kept = ann.history_reads.load(SeqCst);
    assert!(tallies.is_tally(&note)?);
    assert!(!tallies.is_tally(&plain)?);
    assert_eq!(ann.history_reads.load(SeqCst), kept);
    // What has entries is read for all of them in one go.
    let has = |id: &str, field: &str| -> Made<bool> {
        let tally = tallies.by_note(id)?.ok_or("a Tally")?;
        Ok(tally.field(field).ok_or("a field")?.has_entries)
    };
    assert!(has(&note, reps)?);
    assert!(has(&note, reps)?);
    assert_eq!(ann.history_reads.load(SeqCst) - kept, 1);
    // A change counts at once.
    let (other, fields) = tally(&ann, "Run", &["km"], None)?;
    let km = &fields[0];
    assert!(tallies.is_tally(&other)?);
    assert!(!has(&other, km)?);
    tallies.add(&other, &values(&[(km, 5.0)]), "", None)?;
    assert!(has(&other, km)?);
    Ok(())
}

#[test]
fn keys_that_are_no_field_of_a_tally_are_passed_over() -> TestResult {
    let ann = device(1, "Ann")?;
    let (note, fields) = tally(&ann, "Push-ups", &["times"], None)?;
    // As a modified app would.
    for (key, value) in [
        ("tally:name", "Short"),
        ("tally::name", "Empty"),
        ("tally:a:b:name", "Colon"),
    ] {
        ann.append(&note, key, Some(value), None)?;
    }
    let tally = Tallies::new(&ann).by_note(&note)?.ok_or("a Tally")?;
    let ids: Vec<String> = tally.fields.into_iter().map(|f| f.id).collect();
    assert_eq!(ids, fields);
    assert_eq!(Tallies::new(&ann).all()?.len(), 1);
    Ok(())
}

#[test]
fn hidden_values_return_on_restore_and_lines_are_found() -> TestResult {
    let ann = device(1, "Ann")?;
    let tallies = Tallies::new(&ann);
    let (note, fields) = tally(&ann, "Reading", &["pages", "min"], None)?;
    let (pages, minutes) = (&fields[0], &fields[1]);
    let entry = tallies.add(
        &note,
        &values(&[(pages, 12.0), (minutes, 1800.0)]),
        " On the train ",
        None,
    )?;
    assert_eq!(entry.line, "On the train");
    tallies.hide_field(&note, minutes)?;
    let hidden = tallies.by_note(&note)?.ok_or("a Tally")?;
    assert_eq!(hidden.shown().len(), 1);
    tallies.restore_field(&note, minutes)?;
    let chart = tallies.chart(
        &note,
        minutes,
        Some(TallyPeriod::Day),
        &utc_wall,
        &everyone(),
    )?;
    assert_eq!(bars(&chart), [Some(1800.0)]);
    let found: Vec<String> = Search::new(&ann)
        .notes("TRAIN", None)?
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(found, std::slice::from_ref(&note));
    let before = ann.all_entries()?.len();
    tallies.edit_entry(
        &entry.id,
        &BTreeMap::from([(pages.clone(), Some(12.0))]),
        Some("On the train"),
        None,
    )?;
    assert_eq!(ann.all_entries()?.len(), before);
    tallies.trash_entry(&entry.id)?;
    assert!(Search::new(&ann).notes("train", None)?.is_empty());
    Ok(())
}

#[test]
fn removed_ones_are_listed_apart_a_merged_notes_included() -> TestResult {
    let ann = device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let tallies = Tallies::new(&ann);
    let (note, reps) = tally(&ann, "Push-ups", &["times"], None)?;
    let (other, pages) = tally(&ann, "Reading", &["pages"], None)?;
    let older = tallies.add(
        &note,
        &values(&[(&reps[0], 20.0)]),
        "",
        Some(at(2026, 2, 27, 18)?),
    )?;
    let kept = tallies.add(
        &note,
        &values(&[(&reps[0], 25.0)]),
        "",
        Some(at(2026, 2, 28, 18)?),
    )?;
    let elsewhere = tallies.add(&other, &values(&[(&pages[0], 12.0)]), "", None)?;
    let into = notes.capture("Sport", None, None)?;
    notes.merge(&note, &into.id)?;
    let newer = tallies.add(
        &into.id,
        &values(&[(&reps[0], 5.0)]),
        "",
        Some(at(2026, 3, 1, 18)?),
    )?;
    tallies.trash_entry(&newer.id)?;
    tallies.trash_entry(&older.id)?;
    tallies.trash_entry(&elsewhere.id)?;
    let ids = |list: Vec<TallyEntry>| -> Vec<String> { list.into_iter().map(|e| e.id).collect() };
    assert_eq!(
        ids(tallies.removed_entries(&into.id)?),
        [newer.id.clone(), older.id.clone()]
    );
    assert_eq!(ids(tallies.entries(&into.id, &everyone())?), [kept.id]);
    assert!(tallies.removed_entries(&note)?.is_empty());
    tallies.restore_entry(&older.id)?;
    assert_eq!(ids(tallies.removed_entries(&into.id)?), [newer.id]);
    Ok(())
}

#[test]
fn charts_sum_or_take_the_largest_per_day_week_month_and_year() -> TestResult {
    let ann = device(1, "Ann")?;
    let tallies = Tallies::new(&ann);
    let (note, fields) = tally(&ann, "Gym", &["times", "kg"], None)?;
    let (reps, weight) = (&fields[0], &fields[1]);
    // The clock is at Thursday 2026-01-01; its week began on the 29th.
    for (when, r, w) in [
        (at(2025, 12, 29, 8)?, 10.0, 80.0),
        (at(2025, 12, 29, 18)?, 15.0, 82.5),
        (at(2025, 12, 31, 8)?, 20.0, 81.0),
        (at(2026, 1, 1, 8)?, 5.0, 79.0),
        (at(2026, 1, 5, 8)?, 99.0, 99.0),
    ] {
        tallies.add(&note, &values(&[(reps, r), (weight, w)]), "", Some(when))?;
    }
    let chart =
        |field: &str, period| tallies.chart(&note, field, Some(period), &utc_wall, &everyone());
    let days = chart(reps.as_str(), TallyPeriod::Day)?;
    assert_eq!(days[0].start, Day::new(2025, 12, 29));
    assert_eq!(bars(&days), [Some(25.0), None, Some(20.0), Some(5.0)]);
    assert_eq!(days[0].entries, 2);
    let weeks = chart(reps.as_str(), TallyPeriod::Week)?;
    assert_eq!(bars(&weeks), [Some(50.0)]);
    assert_eq!(weeks[0].end, Day::new(2026, 1, 4));
    assert_eq!(
        bars(&chart(reps.as_str(), TallyPeriod::Month)?),
        [Some(45.0), Some(5.0)]
    );
    assert_eq!(
        bars(&chart(weight.as_str(), TallyPeriod::Month)?),
        [Some(82.5), Some(79.0)]
    );
    assert_eq!(
        bars(&chart(reps.as_str(), TallyPeriod::Year)?),
        [Some(45.0), Some(5.0)]
    );
    assert!(chart("none", TallyPeriod::Day)?.is_empty());
    assert_eq!(tallies.entries(&note, &everyone())?.len(), 5);
    Ok(())
}

#[test]
fn a_former_multiplier_reads_its_entries_as_sets() -> TestResult {
    let ann = device(1, "Ann")?;
    let tallies = Tallies::new(&ann);
    let (note, fields) = tally(&ann, "Gym", &["reps", "kg", "rounds"], None)?;
    let (reps, kg, rounds) = (&fields[0], &fields[1], &fields[2]);
    let plain = tallies.by_note(&note)?.ok_or("a Tally")?;
    assert!(!plain.counts_in_sets);
    // As #200 wrote it: Rounds multiplies Reps.
    ann.append(
        &note,
        &keys::field(rounds, keys::MULTIPLIES),
        Some(reps),
        None,
    )?;
    let entry = tallies.add(
        &note,
        &values(&[(reps, 20.0), (kg, 60.0), (rounds, 3.0)]),
        "",
        None,
    )?;
    let read = tallies.by_note(&note)?.ok_or("a Tally")?;
    assert!(read.counts_in_sets);
    let row = values(&[(reps, 20.0), (kg, 60.0)]);
    assert_eq!(read.sets_of(&entry), vec![row.clone(), row.clone(), row]);
    assert_eq!(read.counted(&entry, reps), Some(60.0));
    assert_eq!(read.counted(&entry, kg), Some(60.0));
    assert_eq!(read.counted(&entry, rounds), None);
    let ids: Vec<&str> = read.numbers().iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, [reps.as_str(), kg.as_str()]);
    // Rounded half away from zero, at least one set and at most 99.
    for (times, count) in [(2.5, 3), (0.4, 1), (-2.0, 1), (1000.0, 99)] {
        let e = tallies.add(&note, &values(&[(reps, 1.0), (rounds, times)]), "", None)?;
        assert_eq!(read.sets_of(&e).len(), count, "{times}");
    }
    // Switched off, the entries keep their sets.
    tallies.set_counts_in_sets(&note, false)?;
    let off = tallies.by_note(&note)?.ok_or("a Tally")?;
    assert!(!off.counts_in_sets);
    assert_eq!(off.counted(&entry, reps), Some(60.0));
    // Changed with its sets, it holds them itself.
    tallies.set_counts_in_sets(&note, true)?;
    let row = BTreeMap::from([(reps.clone(), 20.0)]);
    tallies.edit_entry_sets(
        &entry.id,
        &BTreeMap::new(),
        Some(std::slice::from_ref(&row)),
        None,
        None,
    )?;
    let edited = tallies.entry_by_id(&entry.id)?.ok_or("the entry")?;
    assert!(!edited.values.contains_key(rounds));
    assert_eq!(edited.more, vec![row]);
    let read = tallies.by_note(&note)?.ok_or("a Tally")?;
    assert_eq!(read.counted(&edited, reps), Some(40.0));
    Ok(())
}

#[test]
fn sets_are_rows_each_field_totalling_by_its_rule() -> TestResult {
    let ann = device(1, "Ann")?;
    let tallies = Tallies::new(&ann);
    let (note, fields) = tally(&ann, "Bench", &["reps", "kg"], None)?;
    let (reps, kg) = (&fields[0], &fields[1]);
    tallies.set_counts_in_sets(&note, true)?;
    assert_eq!(ann.current(&note, keys::SETS)?.as_deref(), Some("true"));
    let more = [
        values(&[(reps, 8.0), (kg, 65.0)]),
        BTreeMap::new(),
        values(&[(reps, 6.0)]),
    ];
    let entry = tallies.add_with(
        &note,
        &values(&[(reps, 10.0), (kg, 60.0)]),
        &more,
        "",
        Some(at(2026, 1, 1, 7)?),
        |_| Ok(()),
    )?;
    // The set that holds nothing is left out.
    assert_eq!(
        ann.current(&entry.id, &keys::set_value(2, kg))?.as_deref(),
        Some("65")
    );
    assert_eq!(
        ann.current(&entry.id, &keys::set_value(3, reps))?
            .as_deref(),
        Some("6")
    );
    let read = tallies.by_note(&note)?.ok_or("a Tally")?;
    assert_eq!(read.sets_of(&entry).len(), 3);
    assert_eq!(read.counted(&entry, reps), Some(24.0));
    assert_eq!(read.counted(&entry, kg), Some(65.0));
    let chart = tallies.chart(&note, kg, None, &utc_wall, &everyone())?;
    assert_eq!(bars(&chart).first().copied().flatten(), Some(65.0));
    // Fewer sets take the others away.
    let fewer = [values(&[(reps, 9.0)])];
    tallies.edit_entry_sets(&entry.id, &BTreeMap::new(), Some(&fewer), None, None)?;
    assert_eq!(ann.current(&entry.id, &keys::set_value(3, reps))?, None);
    let edited = tallies.entry_by_id(&entry.id)?.ok_or("the entry")?;
    assert_eq!(read.counted(&edited, reps), Some(19.0));
    assert!(
        tallies
            .add_with(
                &note,
                &BTreeMap::new(),
                &[values(&[("x", 1.0)])],
                "",
                None,
                |_| Ok(())
            )
            .is_err()
    );
    let too_many = vec![values(&[(reps, 1.0)]); keys::MAX_SETS];
    assert!(
        tallies
            .add_with(&note, &BTreeMap::new(), &too_many, "", None, |_| Ok(()))
            .is_err()
    );
    Ok(())
}

#[test]
fn a_check_field_is_a_tick_summed_per_bar() -> TestResult {
    let ann = device(1, "Ann")?;
    let tallies = Tallies::new(&ann);
    let (note, fields) = tally(&ann, "Kitchen", &["min"], None)?;
    let minutes = &fields[0];
    let done = tallies.add_check(&note, "Wiped", Some(2.0))?;
    assert_eq!(
        ann.current(&note, &keys::field(&done, keys::KIND))?
            .as_deref(),
        Some("check")
    );
    let tally = tallies.by_note(&note)?.ok_or("a Tally")?;
    let field = tally.field(&done).ok_or("the check")?;
    assert!(field.check && field.unit.is_empty() && field.rule == TallyRule::Sum);
    assert_eq!(field.target, Some(2.0));
    assert!(!tally.targets().contains_key(&done));
    assert!(tallies.set_rule(&note, &done, TallyRule::Max).is_err());
    assert!(tallies.set_unit(&note, &done, "kg").is_err());
    assert!(
        tallies
            .add(&note, &values(&[(&done, 2.0)]), "", None)
            .is_err()
    );
    let in_a_set = [values(&[(&done, 1.0)])];
    assert!(
        tallies
            .add_with(&note, &BTreeMap::new(), &in_a_set, "", None, |_| Ok(()))
            .is_err()
    );
    for hour in [7, 9] {
        tallies.add(
            &note,
            &values(&[(&done, 1.0), (minutes, 5.0)]),
            "",
            Some(at(2026, 1, 1, hour)?),
        )?;
    }
    let entry = tallies.add(&note, &values(&[(minutes, 5.0)]), "", None)?;
    assert_eq!(tally.counted(&entry, &done), None);
    assert!(tally.sets_of(&entry).iter().all(|s| !s.contains_key(&done)));
    let chart = tallies.chart(&note, &done, None, &utc_wall, &everyone())?;
    assert_eq!(bars(&chart).first().copied().flatten(), Some(2.0));
    Ok(())
}

#[test]
fn the_bar_length_lies_on_the_note_and_drives_the_chart() -> TestResult {
    let ann = device(1, "Ann")?;
    let tallies = Tallies::new(&ann);
    let (note, fields) = tally(&ann, "Reading", &["pages"], None)?;
    let pages = &fields[0];
    assert_eq!(
        tallies.by_note(&note)?.ok_or("a Tally")?.bar,
        TallyPeriod::Day
    );
    // A Monday and a Tuesday.
    for day in [29, 30] {
        tallies.add(
            &note,
            &values(&[(pages, 10.0)]),
            "",
            Some(at(2025, 12, day, 7)?),
        )?;
    }
    tallies.set_bar(&note, TallyPeriod::Week)?;
    assert_eq!(ann.current(&note, keys::BAR)?.as_deref(), Some("week"));
    assert_eq!(
        tallies.by_note(&note)?.ok_or("a Tally")?.bar,
        TallyPeriod::Week
    );
    let chart = tallies.chart(&note, pages, None, &utc_wall, &everyone())?;
    assert_eq!(bars(&chart).first().copied().flatten(), Some(20.0));
    let days = tallies.chart(&note, pages, Some(TallyPeriod::Day), &utc_wall, &everyone())?;
    assert_eq!(bars(&days).first().copied().flatten(), Some(10.0));
    ann.append(&note, keys::BAR, Some("fortnight"), None)?;
    assert_eq!(
        tallies.by_note(&note)?.ok_or("a Tally")?.bar,
        TallyPeriod::Day
    );
    assert!(tallies.set_bar("note:none", TallyPeriod::Year).is_err());
    Ok(())
}

#[test]
fn only_the_set_keys_the_apps_write_are_read() {
    assert_eq!(keys::value_of("v:a"), Some((1, "a")));
    assert_eq!(keys::value_of("v2:a"), Some((2, "a")));
    assert_eq!(keys::value_of("v99:a:b"), Some((99, "a:b")));
    for key in ["v1:a", "v02:a", "v100:a", "v2:", "vx:a", "va", "w2:a"] {
        assert_eq!(keys::value_of(key), None, "{key}");
    }
}

#[test]
fn settings_the_cores_read_as_no_multiplier() -> TestResult {
    let ann = device(1, "Ann")?;
    let tallies = Tallies::new(&ann);
    let (note, fields) = tally(&ann, "Gym", &["a", "b", "c"], None)?;
    let (a, b, c) = (&fields[0], &fields[1], &fields[2]);
    // Each other's multipliers, as two Devices might set them.
    ann.append(&note, &keys::field(a, keys::MULTIPLIES), Some(b), None)?;
    ann.append(&note, &keys::field(b, keys::MULTIPLIES), Some(a), None)?;
    ann.append(&note, &keys::field(c, keys::MULTIPLIES), Some(c), None)?;
    let read = tallies.by_note(&note)?.ok_or("a Tally")?;
    assert!(read.fields.iter().all(|f| f.multiplies.is_none()));
    assert!(!read.counts_in_sets);
    Ok(())
}

#[test]
fn everyone_records_entries_and_each_carries_its_author() -> TestResult {
    let (ann, ben, area) = household()?;
    let (note, fields) = tally(&ann, "Push-ups", &["times"], Some(&area))?;
    let reps = &fields[0];
    let own = Tallies::new(&ann).add(&note, &values(&[(reps, 20.0)]), "", None)?;
    exchange(&ben, &ann)?;
    assert_eq!(ben.area_of(&own.id)?, Some(area.clone()));
    let his = Tallies::new(&ben).add(&note, &values(&[(reps, 30.0)]), "Ben", None)?;
    exchange(&ben, &ann)?;
    let tallies = Tallies::new(&ann);
    let ids: Vec<String> = tallies
        .entries(&note, &everyone())?
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(ids, [his.id.clone(), own.id.clone()]);
    assert_eq!(ann.author_of(&his.id)?.map(|p| p.name), Some("Ben".into()));
    assert_eq!(ann.author_of(&own.id)?, None);
    let mine = PersonFilter {
        me: true,
        ..PersonFilter::default()
    };
    let chart = tallies.chart(&note, reps, Some(TallyPeriod::Day), &utc_wall, &mine)?;
    assert_eq!(bars(&chart), [Some(20.0)]);
    let chart = tallies.chart(&note, reps, Some(TallyPeriod::Day), &utc_wall, &everyone())?;
    assert_eq!(bars(&chart), [Some(50.0)]);
    Ok(())
}

#[test]
fn a_member_changes_only_their_own_entries_never_the_fields() -> TestResult {
    let (ann, ben, area) = household()?;
    let (note, fields) = tally(&ann, "Push-ups", &["times"], Some(&area))?;
    let reps = &fields[0];
    let own = Tallies::new(&ann).add(&note, &values(&[(reps, 20.0)]), "", None)?;
    exchange(&ben, &ann)?;
    let by_ben = Tallies::new(&ben);
    let changed = BTreeMap::from([(reps.clone(), Some(1.0))]);
    assert!(by_ben.edit_entry(&own.id, &changed, None, None).is_err());
    assert!(by_ben.rename_field(&note, reps, "Sit-ups").is_err());
    // As a modified app would.
    let forged = [
        ben.append(&own.id, &keys::value(reps), Some("1"), None)?,
        ben.append(&note, &keys::field(reps, keys::NAME), Some("Sit-ups"), None)?,
    ];
    exchange(&ben, &ann)?;
    let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
    assert!(forged.iter().all(|e| held.contains(&e.id())));
    let entry = Tallies::new(&ann).entry_by_id(&own.id)?.ok_or("an entry")?;
    assert_eq!(entry.values.get(reps), Some(&20.0));
    let tally = Tallies::new(&ann).by_note(&note)?.ok_or("a Tally")?;
    assert_eq!(tally.fields[0].name, "times");
    Ok(())
}

#[test]
fn a_tally_recorded_in_before_its_note_was_filed_in_goes_along() -> TestResult {
    let (ann, ben, area) = household()?;
    let (note, fields) = tally(&ann, "Push-ups", &["times"], None)?;
    let entry = Tallies::new(&ann).add(&note, &values(&[(&fields[0], 20.0)]), "", None)?;
    // Written in the Area since, so Ben's vector passes the entry.
    Notes::new(&ann).capture("Milk", Some(&area), None)?;
    exchange(&ben, &ann)?;
    assert_eq!(Tallies::new(&ben).entry_by_id(&entry.id)?, None);
    Notes::new(&ann).file(&note, &area)?;
    exchange(&ben, &ann)?;
    let ids: Vec<String> = Tallies::new(&ben)
        .entries(&note, &everyone())?
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert_eq!(ids, [entry.id]);
    Ok(())
}
