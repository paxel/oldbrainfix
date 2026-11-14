use super::*;
use crate::checklists::tests::{TestResult, exchange, invite, own_exchange, send};
use crate::routines::Routines;
use crate::schedule::{Occurrence, Schedule};
use crate::shared::AreaRole;
use crate::signing::SigningKey;
use crate::{OwnProfile, Places, Search};
use chrono::{Duration, TimeZone};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A clock that moves one second per read from where it is set, in
/// microseconds since the epoch.
#[derive(Clone)]
struct Clock(Arc<AtomicI64>);

impl Clock {
    /// A Thursday.
    fn new() -> Made<Clock> {
        let start = Utc
            .with_ymd_and_hms(2026, 3, 5, 10, 0, 0)
            .single()
            .ok_or("a start")?;
        Ok(Clock(Arc::new(AtomicI64::new(start.timestamp_micros()))))
    }

    /// The next read is `at`.
    fn set(&self, at: DateTime<Utc>) {
        self.0
            .store(at.timestamp_micros() - 1_000_000, AtomicOrdering::SeqCst);
    }

    fn store(&self, seed: u8) -> Made<Store> {
        let ticks = self.0.clone();
        let clock: crate::Clock = Box::new(move || {
            let n = ticks.fetch_add(1_000_000, AtomicOrdering::SeqCst) + 1_000_000;
            DateTime::from_timestamp_micros(n).unwrap_or_default()
        });
        Ok(Store::in_memory(clock, move || {
            Ok(SigningKey::from_seed([seed; 32]))
        })?)
    }

    fn device(&self, seed: u8, name: &str) -> Made<Store> {
        let store = self.store(seed)?;
        OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    }
}

fn utc(day: u32, hour: u32, minute: u32, second: u32) -> Made<DateTime<Utc>> {
    Ok(Utc
        .with_ymd_and_hms(2026, 3, day, hour, minute, second)
        .single()
        .ok_or("a moment")?)
}

/// The wall clock an hour ahead of UTC, as in Central Europe's winter.
fn wall(utc: DateTime<Utc>) -> NaiveDateTime {
    (utc + Duration::hours(1)).naive_utc()
}

fn shopping(store: &Store, text: &str, place: Option<&str>) -> Made<String> {
    let id = Notes::new(store).capture(text, place, None)?.id;
    ShoppingLists::new(store).make(&id)?;
    Ok(id)
}

/// The Lines of the Shopping List of `note` as they show: `~text` for
/// a struck one.
/// Gives each Line of the Shopping List of `note` a week.
fn weekly(store: &Store, note: &str) -> Made<()> {
    let lists = ShoppingLists::new(store);
    for line in lists.by_note(note, &wall)?.ok_or("a Shopping List")?.lines {
        lists.set_reappear(&line.id, Reappear::FALLBACK)?;
    }
    Ok(())
}

fn shown(store: &Store, note: &str) -> Made<Vec<String>> {
    let list = ShoppingLists::new(store).by_note(note, &wall)?;
    Ok(list
        .map(|l| l.lines)
        .unwrap_or_default()
        .iter()
        .map(|l| format!("{}{}", if l.struck { "~" } else { "" }, l.text))
        .collect())
}

fn line_of(store: &Store, note: &str, text: &str) -> Made<ShoppingLine> {
    let list = ShoppingLists::new(store)
        .by_note(note, &wall)?
        .ok_or("a Shopping List")?;
    Ok(list
        .lines
        .into_iter()
        .find(|l| l.text == text)
        .ok_or("a Line")?)
}

fn id_of(store: &Store, note: &str, text: &str) -> Made<String> {
    Ok(line_of(store, note, text)?.id)
}

/// How many entries the log holds for `field` of `entity`.
fn writes(store: &Store, entity: &str, field: &str) -> Made<usize> {
    Ok(store
        .history(entity)?
        .iter()
        .filter(|e| e.field == field)
        .count())
}

#[test]
fn a_checklist_switches_to_a_shopping_list_of_its_lines_the_text_untouched() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = Notes::new(&ann)
        .capture("Trip\nPassport\n Socks\nTickets", None, None)?
        .id;
    let lists = Checklists::new(&ann);
    lists.make(&note)?;
    let third = lists.by_note(&note)?.ok_or("a Checklist")?.lines[2]
        .id
        .clone();
    lists.check(&third)?;
    let texts = writes(&ann, &note, note_keys::TEXT)?;
    let shops = ShoppingLists::new(&ann);
    assert!(shops.may_switch(&note)?);
    shops.switch_list(&note, &wall)?;
    assert!(lists.by_note(&note)?.is_none());
    assert_eq!(shown(&ann, &note)?, ["Passport", "Socks", "~Tickets"]);
    assert_eq!(line_of(&ann, &note, "Socks")?.reappear, Reappear::Unset);
    assert_eq!(writes(&ann, &note, note_keys::TEXT)?, texts, "no text");
    let made = Notes::new(&ann).by_id(&note)?.ok_or("the Note")?;
    assert_eq!(made.text, "Trip");
    Ok(())
}

#[test]
fn a_shopping_list_switches_to_a_checklist_of_the_lines_that_show() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nBread\nEggs", None)?;
    let shops = ShoppingLists::new(&ann);
    shops.set_reappear(&id_of(&ann, &note, "Bread")?, Reappear::Once)?;
    shops.check(&id_of(&ann, &note, "Bread")?, &wall)?;
    shops.check(&id_of(&ann, &note, "Milk")?, &wall)?;
    shops.switch_list(&note, &wall)?;
    assert!(!shops.is_shopping_list(&note)?);
    let list = Checklists::new(&ann).by_note(&note)?.ok_or("a Checklist")?;
    let lines: Vec<String> = list
        .lines
        .iter()
        .map(|l| format!("{}{}", if l.checked { "~" } else { "" }, l.text))
        .collect();
    assert_eq!(lines, ["Eggs", "~Milk"]);
    assert!(list.lines.iter().all(|l| !l.in_group()));
    Ok(())
}

#[test]
fn switching_back_and_forth_starts_a_new_making_each_time() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk", None)?;
    let shops = ShoppingLists::new(&ann);
    let first = shops.making(&note)?.ok_or("a making")?;
    shops.switch_list(&note, &wall)?;
    shops.switch_list(&note, &wall)?;
    assert_ne!(shops.making(&note)?.ok_or("a making")?, first);
    assert_eq!(shown(&ann, &note)?, ["Milk"]);
    assert_eq!(
        ann.entities_with_prefix(checklist_keys::LINE_PREFIX)?.len(),
        3
    );
    Ok(())
}

#[test]
fn neither_a_plain_note_nor_a_tally_switches() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let shops = ShoppingLists::new(&ann);
    let plain = Notes::new(&ann).capture("Shop\nMilk", None, None)?.id;
    assert!(!shops.may_switch(&plain)?);
    assert!(shops.switch_list(&plain, &wall).is_err());
    let tally = Notes::new(&ann).capture("Gym", None, None)?.id;
    Tallies::new(&ann).add_field(&tally, "Reps", "reps", None, None)?;
    assert!(!shops.may_switch(&tally)?);
    Ok(())
}

#[test]
fn the_first_line_stays_and_the_others_become_lines_in_their_order() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = Notes::new(&ann).capture(
        "Groceries\r\nMilk\n  Eggs\n\tTea\n\n✓ Bread\r   \nApples",
        None,
        None,
    )?;
    let lists = ShoppingLists::new(&ann);
    assert!(lists.may_make(&note.id)?);
    lists.make(&note.id)?;
    let made = Notes::new(&ann).by_id(&note.id)?.ok_or("the Note")?;
    assert_eq!(made.text, "Groceries");
    assert!(!made.is_unsorted());
    assert_eq!(
        shown(&ann, &note.id)?,
        ["Milk", "Eggs", "Tea", "Apples", "~Bread"]
    );
    let list = lists.by_note(&note.id, &wall)?.ok_or("a Shopping List")?;
    assert_eq!(
        (list.count(), list.open_count(), list.struck_count()),
        (5, 4, 1)
    );
    for l in &list.lines {
        assert_eq!(l.reappear, Reappear::Unset);
        assert_eq!(ann.current(&l.id, keys::REAPPEAR)?.as_deref(), Some("none"));
        assert_eq!(ann.current(&l.id, checklist_keys::INDENT)?, None);
    }
    assert!(lists.is_shopping_list(&note.id)?);
    assert!(Checklists::new(&ann).by_note(&note.id)?.is_none());
    assert!(!lists.may_make(&note.id)?);
    assert!(lists.make(&note.id).is_err());
    Ok(())
}

#[test]
fn from_a_formatted_text_the_marks_of_items_and_quotes_go() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = Notes::new(&ann).capture_as(
        "**Groceries**\n- Milk\n  * Eggs\n+ ✓ Bread\n12. Tea\n> Apples",
        text_format::MARKDOWN,
        None,
        None,
    )?;
    let lists = ShoppingLists::new(&ann);
    lists.make(&note.id)?;
    let made = Notes::new(&ann).by_id(&note.id)?.ok_or("the Note")?;
    assert_eq!(made.text, "**Groceries**");
    assert_eq!(
        shown(&ann, &note.id)?,
        ["Milk", "Eggs", "Tea", "Apples", "~Bread"]
    );
    // A plain text keeps them as written.
    let plain = Notes::new(&ann).capture("Shop\n- Milk", None, None)?.id;
    lists.make(&plain)?;
    assert_eq!(shown(&ann, &plain)?, ["- Milk"]);
    Ok(())
}

#[test]
fn a_line_came_back_on_the_day_its_check_runs_out() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let lists = ShoppingLists::new(&ann);
    let week = shopping(&ann, "Groceries\nMilk\nEggs\nTea\nRice", None)?;
    weekly(&ann, &week)?;
    let hardware = shopping(&ann, "Hardware\nNails\nGlue", None)?;
    let gone = shopping(&ann, "Gone\nSalt", None)?;
    for text in ["Milk", "Eggs", "Tea"] {
        lists.check(&id_of(&ann, &week, text)?, &wall)?;
    }
    lists.uncheck(&id_of(&ann, &week, "Eggs")?, &wall)?;
    lists.add(&week, "tea", &wall)?;
    lists.set_reappear(
        &id_of(&ann, &hardware, "Nails")?,
        Reappear::After(2, ReappearUnit::Week),
    )?;
    lists.check(&id_of(&ann, &hardware, "Nails")?, &wall)?;
    lists.check(&id_of(&ann, &gone, "Salt")?, &wall)?;
    Notes::new(&ann).trash(&gone)?;
    assert_eq!(line_of(&ann, &week, "Milk")?.came_back_on, None);
    // What came back on `day`, by list, as "list: Line, Line".
    let came_back = |day: Day| -> Made<Vec<String>> {
        let mut out = Vec::new();
        for note in [&week, &hardware] {
            let list = lists.by_note(note, &wall)?.ok_or("a Shopping List")?;
            let texts: Vec<&str> = list
                .came_back(day)
                .iter()
                .map(|l| l.text.as_str())
                .collect();
            if !texts.is_empty() {
                out.push(format!("{note}: {}", texts.join(", ")));
            }
        }
        Ok(out)
    };
    assert!(came_back(Day::new(2026, 3, 11))?.is_empty());
    // Midnight on the wall: Thursday the 12th begins.
    clock.set(utc(11, 23, 0, 0)?);
    let back = Day::new(2026, 3, 12);
    assert_eq!(line_of(&ann, &week, "Milk")?.came_back_on, Some(back));
    for text in ["Eggs", "Tea", "Rice"] {
        assert_eq!(line_of(&ann, &week, text)?.came_back_on, None, "{text}");
    }
    assert_eq!(came_back(back)?, [format!("{week}: Milk")]);
    // The day after it is still open, but no longer back today.
    clock.set(utc(12, 23, 0, 0)?);
    assert_eq!(line_of(&ann, &week, "Milk")?.came_back_on, Some(back));
    assert!(came_back(Day::new(2026, 3, 13))?.is_empty());
    Ok(())
}

#[test]
fn a_note_is_at_most_one_of_a_tally_a_checklist_and_a_shopping_list() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let lists = ShoppingLists::new(&ann);
    let gym = Notes::new(&ann).capture("Gym\nPush-ups", None, None)?.id;
    Tallies::new(&ann).add_field(&gym, "Push-ups", "", None, None)?;
    assert!(!lists.may_make(&gym)?);
    assert!(lists.make(&gym).is_err());
    let packing = Notes::new(&ann).capture("Packing\nSocks", None, None)?.id;
    Checklists::new(&ann).make(&packing)?;
    assert!(!lists.may_make(&packing)?);
    assert!(lists.by_note(&packing, &wall)?.is_none());
    let shop = shopping(&ann, "Shop\nMilk", None)?;
    assert!(!Checklists::new(&ann).may_make(&shop)?);
    assert!(Checklists::new(&ann).make(&shop).is_err());
    assert!(Checklists::new(&ann).by_note(&shop)?.is_none());
    assert!(
        Checklists::new(&ann)
            .add(&shop, "Tea", None, false)
            .is_err()
    );
    assert!(
        Tallies::new(&ann)
            .add_field(&shop, "Milk", "", None, None)
            .is_err()
    );
    // Made one on two Devices apart: the Tally wins.
    let both = shopping(&ann, "Both\nMilk", None)?;
    let field = |key| crate::tallies::keys::field("f1", key);
    ann.append(
        &both,
        &field(crate::tallies::keys::NAME),
        Some("Milk"),
        None,
    )?;
    ann.append(&both, &field(crate::tallies::keys::UNIT), Some("l"), None)?;
    assert!(Tallies::new(&ann).is_tally(&both)?);
    assert!(lists.by_note(&both, &wall)?.is_none());
    assert!(!lists.is_shopping_list(&both)?);
    Ok(())
}

#[test]
fn kinds_and_reappear_times_no_app_writes_are_read_without_fail() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = Notes::new(&ann).capture("Odd\nOne", None, None)?.id;
    for kind in ["shopping", "shopping:", "shopping:a:b"] {
        ann.append(&note, checklist_keys::KIND, Some(kind), None)?;
        assert!(
            ShoppingLists::new(&ann).by_note(&note, &wall)?.is_none(),
            "{kind}"
        );
    }
    ann.append(&note, checklist_keys::KIND, None, None)?;
    ShoppingLists::new(&ann).make(&note)?;
    let one = id_of(&ann, &note, "One")?;
    for code in [Some("0"), Some("5"), Some("x"), Some(""), Some("1W"), None] {
        ann.append(&one, keys::REAPPEAR, code, None)?;
        assert_eq!(
            line_of(&ann, &note, "One")?.reappear,
            Reappear::After(1, ReappearUnit::Week),
            "{code:?}"
        );
    }
    assert_eq!(Reappear::read(Some("once")), Reappear::Once);
    assert_eq!(
        Reappear::read(Some("4w")),
        Reappear::After(4, ReappearUnit::Week)
    );
    Ok(())
}

fn at(year: i32, month: u32, day: u32, hour: u32) -> Made<DateTime<Utc>> {
    Ok(Utc
        .with_ymd_and_hms(year, month, day, hour, 0, 0)
        .single()
        .ok_or("a moment")?)
}

fn after(count: u32, unit: ReappearUnit) -> Made<Reappear> {
    Ok(Reappear::after(count, unit).ok_or("a count")?)
}

#[test]
fn a_new_line_stays_struck_until_a_time_is_set_which_counts_from_its_check() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs", None)?;
    let lists = ShoppingLists::new(&ann);
    let milk = id_of(&ann, &note, "Milk")?;
    assert_eq!(line_of(&ann, &note, "Milk")?.reappear, Reappear::Unset);
    lists.check(&milk, &wall)?;
    assert!(line_of(&ann, &note, "Milk")?.struck);
    assert_eq!(line_of(&ann, &note, "Milk")?.back_on, None);
    // A year on, still struck.
    clock.set(at(2027, 3, 5, 10)?);
    assert_eq!(shown(&ann, &note)?, ["Eggs", "~Milk"]);
    // Ten days after the check, a week makes it back since the 12th.
    clock.set(at(2026, 3, 15, 10)?);
    lists.set_reappear(&milk, after(1, ReappearUnit::Week)?)?;
    assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs"]);
    assert_eq!(
        line_of(&ann, &note, "Milk")?.came_back_on,
        Some(Day::new(2026, 3, 12))
    );
    // A month instead: struck again until the 5th of April.
    lists.set_reappear(&milk, after(1, ReappearUnit::Month)?)?;
    assert_eq!(shown(&ann, &note)?, ["Eggs", "~Milk"]);
    assert_eq!(
        line_of(&ann, &note, "Milk")?.back_on,
        Some(Day::new(2026, 4, 5))
    );
    lists.set_reappear(&milk, Reappear::Unset)?;
    assert_eq!(ann.current(&milk, keys::REAPPEAR)?.as_deref(), Some("none"));
    assert_eq!(line_of(&ann, &note, "Milk")?.back_on, None);
    Ok(())
}

#[test]
fn days_and_weeks_count_on_and_months_and_years_go_by_the_calendar() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    // Saturday the 31st of January on the wall.
    clock.set(at(2026, 1, 31, 10)?);
    let note = shopping(&ann, "Shop\nA\nB\nC\nD\nE\nF", None)?;
    let lists = ShoppingLists::new(&ann);
    let times = [
        ("A", after(1, ReappearUnit::Day)?, Day::new(2026, 2, 1)),
        ("B", after(6, ReappearUnit::Week)?, Day::new(2026, 3, 14)),
        ("C", after(1, ReappearUnit::Month)?, Day::new(2026, 2, 28)),
        ("D", after(13, ReappearUnit::Month)?, Day::new(2027, 2, 28)),
        ("E", after(1, ReappearUnit::Year)?, Day::new(2027, 1, 31)),
        ("F", after(99, ReappearUnit::Day)?, Day::new(2026, 5, 10)),
    ];
    for (text, time, _) in times {
        lists.set_reappear(&id_of(&ann, &note, text)?, time)?;
        lists.check(&id_of(&ann, &note, text)?, &wall)?;
    }
    for (text, _, back) in times {
        assert_eq!(line_of(&ann, &note, text)?.back_on, Some(back), "{text}");
    }
    // At the start of the 28th of February on the wall, C is back.
    clock.set(at(2026, 2, 27, 23)?);
    assert!(!line_of(&ann, &note, "C")?.struck);
    assert_eq!(
        line_of(&ann, &note, "C")?.came_back_on,
        Some(Day::new(2026, 2, 28))
    );
    assert!(line_of(&ann, &note, "B")?.struck);
    // A leap day a year on is the 28th, four years on the 29th.
    let leap = Day::new(2028, 2, 29);
    let back = |count, unit, day| -> Made<Option<Day>> { Ok(after(count, unit)?.back_from(day)) };
    assert_eq!(
        back(1, ReappearUnit::Year, leap)?,
        Some(Day::new(2029, 2, 28))
    );
    assert_eq!(
        back(4, ReappearUnit::Year, leap)?,
        Some(Day::new(2032, 2, 29))
    );
    assert_eq!(
        back(1, ReappearUnit::Month, leap)?,
        Some(Day::new(2028, 3, 29))
    );
    assert_eq!(
        back(11, ReappearUnit::Month, Day::new(2026, 3, 31))?,
        Some(Day::new(2027, 2, 28))
    );
    assert_eq!(Reappear::Unset.back_from(leap), None);
    assert_eq!(Reappear::Once.back_from(leap), None);
    Ok(())
}

#[test]
fn the_week_values_of_before_are_kept_and_reading_writes_nothing() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs\nTea", None)?;
    let lists = ShoppingLists::new(&ann);
    // As a version before #195 wrote them.
    for (text, code) in [("Milk", Some("1w")), ("Eggs", Some("4w")), ("Tea", None)] {
        ann.append(&id_of(&ann, &note, text)?, keys::REAPPEAR, code, None)?;
    }
    for text in ["Milk", "Eggs", "Tea"] {
        lists.check(&id_of(&ann, &note, text)?, &wall)?;
    }
    let before = ann.all_entries()?.len();
    assert_eq!(
        line_of(&ann, &note, "Milk")?.reappear,
        after(1, ReappearUnit::Week)?
    );
    assert_eq!(
        line_of(&ann, &note, "Eggs")?.reappear,
        after(4, ReappearUnit::Week)?
    );
    assert_eq!(line_of(&ann, &note, "Tea")?.reappear, Reappear::FALLBACK);
    assert_eq!(
        line_of(&ann, &note, "Milk")?.back_on,
        Some(Day::new(2026, 3, 12))
    );
    assert_eq!(
        line_of(&ann, &note, "Eggs")?.back_on,
        Some(Day::new(2026, 4, 2))
    );
    assert_eq!(
        line_of(&ann, &note, "Tea")?.back_on,
        Some(Day::new(2026, 3, 12))
    );
    assert_eq!(ann.all_entries()?.len(), before);
    let eggs = id_of(&ann, &note, "Eggs")?;
    assert_eq!(ann.current(&eggs, keys::REAPPEAR)?.as_deref(), Some("4w"));
    Ok(())
}

#[test]
fn reappear_codes_as_stored_and_those_no_app_writes_read_as_a_week() -> TestResult {
    let codes = [
        ("none", Reappear::Unset),
        ("once", Reappear::Once),
        ("1d", after(1, ReappearUnit::Day)?),
        ("20w", after(20, ReappearUnit::Week)?),
        ("3m", after(3, ReappearUnit::Month)?),
        ("99y", after(99, ReappearUnit::Year)?),
    ];
    for (code, time) in codes {
        assert_eq!(Reappear::read(Some(code)), time, "{code}");
        assert_eq!(time.code(), code);
    }
    let odd = [
        None,
        Some(""),
        Some("0w"),
        Some("01w"),
        Some("100d"),
        Some("1W"),
        Some(" 1w"),
        Some("1w "),
        Some("w1"),
        Some("1 w"),
        Some("5"),
        Some("1x"),
        Some("٣w"),
        Some("None"),
    ];
    for code in odd {
        assert_eq!(Reappear::read(code), Reappear::FALLBACK, "{code:?}");
    }
    assert_eq!(Reappear::FALLBACK, after(1, ReappearUnit::Week)?);
    assert_eq!(Reappear::after(0, ReappearUnit::Day), None);
    assert_eq!(Reappear::after(100, ReappearUnit::Day), None);
    Ok(())
}

#[test]
fn a_checked_line_is_struck_until_the_start_of_its_day_back_by_the_wall_clock() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs\nTea", None)?;
    weekly(&ann, &note)?;
    let lists = ShoppingLists::new(&ann);
    lists.check(&id_of(&ann, &note, "Milk")?, &wall)?;
    assert_eq!(shown(&ann, &note)?, ["Eggs", "Tea", "~Milk"]);
    let milk = line_of(&ann, &note, "Milk")?;
    assert_eq!(milk.back_on, Some(Day::new(2026, 3, 12)));
    assert!(milk.checked_at.is_some());
    let before = ann.all_entries()?.len();
    clock.set(utc(11, 22, 59, 59)?);
    assert_eq!(shown(&ann, &note)?, ["Eggs", "Tea", "~Milk"]);
    clock.set(utc(11, 23, 0, 0)?);
    assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Tea"]);
    assert_eq!(line_of(&ann, &note, "Milk")?.back_on, None);
    assert_eq!(ann.all_entries()?.len(), before, "reading writes none");
    // 22:30 UTC on Wednesday is 00:30 on Thursday two hours ahead.
    let two_ahead = |utc: DateTime<Utc>| (utc + Duration::hours(2)).naive_utc();
    let late = shopping(&ann, "Late\nSalt", None)?;
    weekly(&ann, &late)?;
    clock.set(utc(18, 22, 30, 0)?);
    lists.check(&id_of(&ann, &late, "Salt")?, &two_ahead)?;
    let back = |wall: &WallClock| -> Made<Option<Day>> {
        let list = lists.by_note(&late, wall)?.ok_or("a Shopping List")?;
        Ok(list.lines.first().and_then(|l| l.back_on))
    };
    assert_eq!(back(&two_ahead)?, Some(Day::new(2026, 3, 26)));
    assert_eq!(back(&wall)?, Some(Day::new(2026, 3, 25)));
    clock.set(utc(25, 21, 59, 59)?);
    assert_eq!(back(&two_ahead)?, Some(Day::new(2026, 3, 26)));
    clock.set(utc(25, 22, 0, 0)?);
    assert_eq!(back(&two_ahead)?, None);
    Ok(())
}

#[test]
fn each_reappear_time_and_once_is_gone() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nOnce\nOne\nTwo\nThree\nFour", None)?;
    weekly(&ann, &note)?;
    let lists = ShoppingLists::new(&ann);
    let once = id_of(&ann, &note, "Once")?;
    lists.set_reappear(&once, Reappear::Once)?;
    lists.set_reappear(
        &id_of(&ann, &note, "Two")?,
        Reappear::After(2, ReappearUnit::Week),
    )?;
    lists.set_reappear(
        &id_of(&ann, &note, "Three")?,
        Reappear::After(3, ReappearUnit::Week),
    )?;
    lists.set_reappear(
        &id_of(&ann, &note, "Four")?,
        Reappear::After(4, ReappearUnit::Week),
    )?;
    for text in ["Once", "One", "Two", "Three", "Four"] {
        lists.check(&id_of(&ann, &note, text)?, &wall)?;
    }
    assert_eq!(shown(&ann, &note)?, ["~One", "~Two", "~Three", "~Four"]);
    let list = lists.by_note(&note, &wall)?.ok_or("a Shopping List")?;
    let days: Vec<Option<Day>> = list.lines.iter().map(|l| l.back_on).collect();
    assert_eq!(days, [12, 19, 26, 33].map(|d| Some(Day::new(2026, 3, d))));
    assert_eq!(ann.current(&once, checklist_keys::TRASHED)?, None);
    assert!(lists.check(&once, &wall).is_err());
    clock.set(utc(25, 23, 0, 0)?);
    assert_eq!(shown(&ann, &note)?, ["One", "Two", "Three", "~Four"]);
    Ok(())
}

#[test]
fn the_open_lines_keep_their_order_and_the_struck_one_checked_last_is_at_the_end() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs\nTea\nRice", None)?;
    weekly(&ann, &note)?;
    let lists = ShoppingLists::new(&ann);
    lists.check(&id_of(&ann, &note, "Eggs")?, &wall)?;
    lists.check(&id_of(&ann, &note, "Milk")?, &wall)?;
    lists.check(&id_of(&ann, &note, "Rice")?, &wall)?;
    assert_eq!(shown(&ann, &note)?, ["Tea", "~Eggs", "~Milk", "~Rice"]);
    // Unchecked, it comes back now, in its place.
    lists.uncheck(&id_of(&ann, &note, "Rice")?, &wall)?;
    assert_eq!(shown(&ann, &note)?, ["Tea", "Rice", "~Eggs", "~Milk"]);
    let before = ann.all_entries()?.len();
    lists.check(&id_of(&ann, &note, "Eggs")?, &wall)?;
    lists.uncheck(&id_of(&ann, &note, "Tea")?, &wall)?;
    assert_eq!(ann.all_entries()?.len(), before);
    // Eggs and Milk come back on the same day, each in its place.
    clock.set(utc(11, 23, 0, 0)?);
    assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Tea", "Rice"]);
    Ok(())
}

#[test]
fn a_line_without_a_valid_key_shows_after_the_others_the_one_added_last_first() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs", None)?;
    let lists = ShoppingLists::new(&ann);
    let making = lists
        .by_note(&note, &wall)?
        .ok_or("a Shopping List")?
        .making;
    // As a modified app would.
    for (text, order) in [("Old", None), ("Odd", Some("é")), ("New", Some(""))] {
        let id = format!("line:{}", ann.new_id()?);
        ann.append(
            &id,
            checklist_keys::TYPE,
            Some(checklist_keys::TYPE_LINE),
            None,
        )?;
        ann.append(&id, checklist_keys::NOTE, Some(&note), None)?;
        ann.append(&id, checklist_keys::MADE, Some(&making), None)?;
        let text = StoredText::plain(text).encode();
        ann.append(&id, checklist_keys::TEXT, Some(&text), None)?;
        if let Some(order) = order {
            ann.append(&id, checklist_keys::ORDER, Some(order), None)?;
        }
    }
    lists.add(&note, "Tea", &wall)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["Milk", "Eggs", "Tea", "New", "Odd", "Old"]
    );
    Ok(())
}

#[test]
fn a_new_name_comes_in_where_it_is_told_before_the_struck_ones_after_the_line_above() -> TestResult
{
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs\nHam\nJam\nTea", None)?;
    weekly(&ann, &note)?;
    let lists = ShoppingLists::new(&ann);
    for text in ["Eggs", "Ham", "Jam"] {
        lists.check(&id_of(&ann, &note, text)?, &wall)?;
    }
    lists.add_at(&note, "Bread", &wall, Some(1))?;
    lists.add_at(&note, "Rice", &wall, Some(0))?;
    lists.add_at(&note, "Salt", &wall, Some(9))?;
    assert_eq!(
        shown(&ann, &note)?,
        [
            "Rice", "Milk", "Bread", "Tea", "Salt", "~Eggs", "~Ham", "~Jam"
        ]
    );
    // A name it has stays where it is.
    lists.add_at(&note, "tea", &wall, Some(0))?;
    clock.set(utc(11, 23, 0, 0)?);
    assert_eq!(
        shown(&ann, &note)?,
        ["Rice", "Milk", "Bread", "Eggs", "Ham", "Jam", "Tea", "Salt"]
    );
    Ok(())
}

#[test]
fn an_open_line_moves_before_another_or_after_and_a_struck_one_keeps_its_place() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nBread\nHam\nJam\nEggs\nTea\nSoap", None)?;
    weekly(&ann, &note)?;
    let lists = ShoppingLists::new(&ann);
    let id = |text: &str| id_of(&ann, &note, text);
    for text in ["Bread", "Ham", "Jam"] {
        lists.check(&id(text)?, &wall)?;
    }
    lists.move_line(&id("Soap")?, &wall, Some(&id("Eggs")?))?;
    lists.move_line(&id("Tea")?, &wall, Some(&id("Milk")?))?;
    assert_eq!(
        shown(&ann, &note)?,
        ["Tea", "Milk", "Soap", "Eggs", "~Bread", "~Ham", "~Jam"]
    );
    lists.move_line(&id("Milk")?, &wall, None)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["Tea", "Soap", "Eggs", "Milk", "~Bread", "~Ham", "~Jam"]
    );
    // The struck ones lay between Milk and Eggs, so Soap went before
    // them.
    clock.set(utc(11, 23, 0, 0)?);
    assert_eq!(
        shown(&ann, &note)?,
        ["Tea", "Soap", "Bread", "Ham", "Jam", "Eggs", "Milk"]
    );
    Ok(())
}

#[test]
fn a_struck_line_does_not_move_and_where_a_line_is_already_nothing_is_written() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs\nTea\nJam", None)?;
    let lists = ShoppingLists::new(&ann);
    let milk = id_of(&ann, &note, "Milk")?;
    let eggs = id_of(&ann, &note, "Eggs")?;
    let jam = id_of(&ann, &note, "Jam")?;
    let tea = id_of(&ann, &note, "Tea")?;
    lists.check(&tea, &wall)?;
    assert!(lists.move_line(&tea, &wall, None).is_err());
    assert!(lists.move_line(&milk, &wall, Some(&tea)).is_err());
    assert!(lists.move_line(&milk, &wall, Some("line:none")).is_err());
    let before = ann.all_entries()?.len();
    lists.move_line(&milk, &wall, Some(&milk))?;
    lists.move_line(&milk, &wall, Some(&eggs))?;
    lists.move_line(&jam, &wall, None)?;
    assert_eq!(ann.all_entries()?.len(), before);
    lists.move_line(&jam, &wall, Some(&milk))?;
    assert_eq!(shown(&ann, &note)?, ["Jam", "Milk", "Eggs", "~Tea"]);
    lists.delete(&milk)?;
    assert!(lists.move_line(&milk, &wall, None).is_err());
    Ok(())
}

#[test]
fn a_name_it_has_adds_nothing_and_brings_a_struck_or_gone_line_back() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs\nÄpfel\nTea", None)?;
    let lists = ShoppingLists::new(&ann);
    let milk = id_of(&ann, &note, "Milk")?;
    let tea = id_of(&ann, &note, "Tea")?;
    lists.set_reappear(&tea, Reappear::Once)?;
    lists.check(&milk, &wall)?;
    lists.check(&tea, &wall)?;
    assert_eq!(shown(&ann, &note)?, ["Eggs", "Äpfel", "~Milk"]);
    let before = ann.all_entries()?.len();
    assert_eq!(
        lists.add(&note, " eggs ", &wall)?,
        id_of(&ann, &note, "Eggs")?
    );
    assert_eq!(
        lists.add(&note, "äPFEL", &wall)?,
        id_of(&ann, &note, "Äpfel")?
    );
    assert_eq!(ann.all_entries()?.len(), before, "open ones stay");
    assert_eq!(lists.add(&note, "MILK", &wall)?, milk);
    assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Äpfel"]);
    assert_eq!(lists.add(&note, "tea", &wall)?, tea);
    assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Äpfel", "Tea"]);
    assert_eq!(line_of(&ann, &note, "Tea")?.reappear, Reappear::Once);
    // A new name comes in at the end; a deleted one is not brought back.
    let rice = lists.add(&note, "  Rice\n", &wall)?;
    assert_eq!(shown(&ann, &note)?.last().map(String::as_str), Some("Rice"));
    lists.delete(&rice)?;
    assert_ne!(lists.add(&note, "rice", &wall)?, rice);
    assert!(lists.add(&note, "  ", &wall).is_err());
    Ok(())
}

#[test]
fn the_lists_to_add_to_are_those_not_in_the_trash_oldest_first() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let groceries = shopping(&ann, "Groceries", None)?;
    notes.capture("plain", None, None)?;
    let gone = shopping(&ann, "Gone", None)?;
    let hardware = shopping(&ann, "Hardware", None)?;
    let packing = notes.capture("Packing\nSocks", None, None)?.id;
    Checklists::new(&ann).make(&packing)?;
    notes.trash(&gone)?;
    let ids: Vec<String> = ShoppingLists::new(&ann)
        .addable()?
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(ids, [groceries, hardware]);
    Ok(())
}

#[test]
fn the_lines_of_a_text_come_in_at_once_in_their_order_at_the_end() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nTea", None)?;
    let lists = ShoppingLists::new(&ann);
    let milk = id_of(&ann, &note, "Milk")?;
    let tea = id_of(&ann, &note, "Tea")?;
    lists.check(&milk, &wall)?;
    let ids = lists.add_lines(&note, "Eggs\r\n  milk \n\n Tea\rbread", &wall)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["Milk", "Tea", "Eggs", "bread"],
        "an open one stays"
    );
    assert_eq!(
        ids,
        [
            id_of(&ann, &note, "Eggs")?,
            milk,
            tea,
            id_of(&ann, &note, "bread")?
        ]
    );
    assert_eq!(ann.entities_of_type(checklist_keys::TYPE_LINE)?.len(), 4);
    assert!(lists.add_lines(&note, " \n ", &wall)?.is_empty());
    let plain = Notes::new(&ann).capture("plain", None, None)?.id;
    assert!(lists.add_lines(&plain, "Milk", &wall).is_err());
    Ok(())
}

#[test]
fn lines_are_renamed_deleted_restored_and_go_back_into_a_plain_note() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs\nTea", None)?;
    let lists = ShoppingLists::new(&ann);
    let milk = id_of(&ann, &note, "Milk")?;
    lists.rename(&milk, "Oat milk")?;
    let before = ann.all_entries()?.len();
    lists.rename(&milk, " Oat milk ")?;
    lists.set_reappear(&milk, Reappear::Unset)?;
    assert_eq!(ann.all_entries()?.len(), before);
    lists.delete(&milk)?;
    assert_eq!(shown(&ann, &note)?, ["Eggs", "Tea"]);
    assert!(lists.rename(&milk, "x").is_err());
    lists.restore(&milk)?;
    assert!(lists.restore(&milk).is_err());
    assert_eq!(shown(&ann, &note)?, ["Oat milk", "Eggs", "Tea"]);
    lists.set_reappear(&id_of(&ann, &note, "Tea")?, Reappear::Once)?;
    lists.check(&id_of(&ann, &note, "Tea")?, &wall)?;
    lists.check(&milk, &wall)?;
    // Search finds it by its Lines, not by those gone.
    assert_eq!(Search::new(&ann).notes("MILK", None)?.len(), 1);
    assert!(Search::new(&ann).notes("tea", None)?.is_empty());
    lists.to_plain(&note, &wall)?;
    assert!(!lists.is_shopping_list(&note)?);
    let plain = Notes::new(&ann).by_id(&note)?.ok_or("the Note")?;
    assert_eq!(plain.text, "Shop\nEggs\n✓ Oat milk");
    lists.make(&note)?;
    assert_eq!(shown(&ann, &note)?, ["Eggs", "~Oat milk"]);
    Ok(())
}

#[test]
fn the_deleted_lines_are_listed_the_last_deleted_first_while_a_restore_brings_them_back()
-> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs\nTea", None)?;
    let lists = ShoppingLists::new(&ann);
    assert!(lists.deleted_lines(&note)?.is_empty());
    let milk = id_of(&ann, &note, "Milk")?;
    let tea = id_of(&ann, &note, "Tea")?;
    lists.delete(&milk)?;
    lists.delete(&tea)?;
    let texts =
        |lists: Vec<DeletedLine>| -> Vec<String> { lists.into_iter().map(|l| l.text).collect() };
    assert_eq!(texts(lists.deleted_lines(&note)?), ["Tea", "Milk"]);
    lists.restore(&milk)?;
    assert_eq!(texts(lists.deleted_lines(&note)?), ["Tea"]);
    assert!(Checklists::new(&ann).deleted_lines(&note)?.is_empty());
    lists.to_plain(&note, &wall)?;
    lists.make(&note)?;
    assert!(lists.deleted_lines(&note)?.is_empty());
    Ok(())
}

#[test]
fn every_member_checks_and_adds_and_the_rest_stays_with_the_author_and_the_managers() -> TestResult
{
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let ben = clock.device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    invite(&ann, &ben, &area)?;
    let note = shopping(&ann, "Shop\nMilk\nEggs", Some(&area))?;
    exchange(&ben, &ann)?;
    let lists = ShoppingLists::new(&ben);
    let milk = id_of(&ben, &note, "Milk")?;
    let eggs = id_of(&ben, &note, "Eggs")?;
    assert!(lists.may_check(&note)?);
    lists.check(&milk, &wall)?;
    lists.check(&eggs, &wall)?;
    lists.add(&note, "milk", &wall)?;
    let bread = lists.add(&note, "Bread", &wall)?;
    exchange(&ben, &ann)?;
    assert_eq!(shown(&ann, &note)?, ["Milk", "Bread", "~Eggs"]);
    assert!(lists.may_edit(&bread)?);
    assert!(!lists.may_edit(&milk)?);
    assert!(lists.rename(&milk, "Beer").is_err());
    assert!(lists.delete(&milk).is_err());
    assert!(lists.set_reappear(&milk, Reappear::Once).is_err());
    assert!(lists.to_plain(&note, &wall).is_err());
    // As a modified app would.
    let forged = [
        ben.append(&milk, keys::REAPPEAR, Some("once"), None)?,
        ben.append(
            &eggs,
            checklist_keys::TRASHED,
            Some("2026-03-05T11:00:00.000Z"),
            None,
        )?,
    ];
    exchange(&ben, &ann)?;
    let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
    assert!(forged.iter().all(|e| held.contains(&e.id())));
    assert_eq!(line_of(&ann, &note, "Milk")?.reappear, Reappear::Unset);
    assert_eq!(shown(&ann, &note)?, ["Milk", "Bread", "~Eggs"]);
    let grants = ann.grants_of(&area)?.ok_or("the grants")?;
    let person = grants
        .person_of(&ben.device_id()?)
        .ok_or("Ben")?
        .to_string();
    ann.set_role(&area, &person, AreaRole::Manager)?;
    exchange(&ben, &ann)?;
    lists.set_reappear(&milk, Reappear::After(2, ReappearUnit::Week))?;
    exchange(&ben, &ann)?;
    assert_eq!(
        line_of(&ann, &note, "Milk")?.reappear,
        Reappear::After(2, ReappearUnit::Week)
    );
    Ok(())
}

#[test]
fn a_member_moves_their_own_lines_not_another_persons() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let ben = clock.device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    invite(&ann, &ben, &area)?;
    let note = shopping(&ann, "Shop\nMilk\nEggs\nTea", Some(&area))?;
    exchange(&ben, &ann)?;
    let lists = ShoppingLists::new(&ben);
    let milk = id_of(&ben, &note, "Milk")?;
    let bread = lists.add(&note, "Bread", &wall)?;
    assert!(lists.move_line(&milk, &wall, None).is_err());
    lists.move_line(&bread, &wall, Some(&milk))?;
    // As a modified app would.
    let forged = ben.append(&milk, checklist_keys::ORDER, Some("zz"), None)?;
    exchange(&ben, &ann)?;
    let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
    assert!(held.contains(&forged.id()));
    assert_eq!(shown(&ann, &note)?, ["Bread", "Milk", "Eggs", "Tea"]);
    ShoppingLists::new(&ann).move_line(&bread, &wall, None)?;
    assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Tea", "Bread"]);
    Ok(())
}

#[test]
fn two_renames_of_one_line_are_a_conflict_and_a_later_check_wins() -> TestResult {
    let clock = Clock::new()?;
    let phone = clock.store(1)?;
    let desk = clock.store(2)?;
    let note = shopping(&phone, "Shop\nMilk\nEggs", None)?;
    send(&phone, &desk)?;
    let milk = id_of(&phone, &note, "Milk")?;
    let eggs = id_of(&phone, &note, "Eggs")?;
    ShoppingLists::new(&phone).rename(&milk, "Oat milk")?;
    ShoppingLists::new(&desk).rename(&milk, "Soy milk")?;
    ShoppingLists::new(&desk).check(&eggs, &wall)?;
    ShoppingLists::new(&phone).check(&eggs, &wall)?;
    ShoppingLists::new(&phone).uncheck(&eggs, &wall)?;
    send(&phone, &desk)?;
    send(&desk, &phone)?;
    let conflicts = desk.conflicts()?;
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].entity, milk);
    assert_eq!(conflicts[0].entity_type, checklist_keys::TYPE_LINE);
    assert!(desk.may_resolve(&milk, &conflicts[0].entity_type)?);
    // Back in its place.
    assert_eq!(
        shown(&desk, &note)?.last().map(String::as_str),
        Some("Eggs")
    );
    assert_eq!(shown(&phone, &note)?, shown(&desk, &note)?);
    Ok(())
}

#[test]
fn a_routine_spawned_from_a_shopping_list_is_done_only_when_checked_off() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk", None)?;
    let routines = Routines::new(&ann);
    let schedule = Schedule::new(Day::new(2026, 3, 5), &[], None).ok_or("a schedule")?;
    let routine = routines.spawn(&note, &schedule)?.id;
    let day = Occurrence::new(Day::new(2026, 3, 5), None);
    let milk = id_of(&ann, &note, "Milk")?;
    ShoppingLists::new(&ann).check(&milk, &wall)?;
    // As a modified app would.
    ann.append(
        &routine,
        &checklist_keys::check_for(&day, &milk),
        Some("2026-03-05T10:30:00.000Z"),
        None,
    )?;
    let done = |o: &Occurrence| -> Made<Option<DateTime<Utc>>> {
        Ok(routines.by_id(&routine)?.ok_or("the Routine")?.done_at(o))
    };
    assert_eq!(done(&day)?, None);
    let on = crate::RoutineOccurrence {
        routine_id: routine.clone(),
        occurrence: day,
    };
    assert!(Checklists::new(&ann).for_occurrence(&on)?.is_none());
    routines.mark_done(&routine, &day, None)?;
    assert!(done(&day)?.is_some());
    Ok(())
}

#[test]
fn a_shopping_list_made_of_a_received_checklist_stays_one() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let ben = clock.device(2, "Ben")?;
    let note = Notes::new(&ann).capture("Shop\nMilk", None, None)?.id;
    Checklists::new(&ann).make(&note)?;
    let item = ann.sent_item_of(&note, false)?;
    let got = ben.accept_sent_item(&item, false)?.ok_or("accepted")?;
    Checklists::new(&ben).to_plain(&got)?;
    ShoppingLists::new(&ben).make(&got)?;
    Checklists::new(&ann).add(&note, "Tea", None, false)?;
    let key = ann.signing_key()?;
    let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
    let item = ann.sent_item_of(&note, false)?;
    let file = crate::sent::encode(&item, &key, &[], &mut random)?;
    let opened = ben.open_sent_item(&file)?.map_err(|_| "a Sent Item")?;
    let changes = opened.changes.ok_or("an update")?;
    assert!(!changes.makes_checklist);
    assert!(changes.lines_added.is_empty());
    ben.accept_sent_item(&opened.item, false)?;
    assert_eq!(shown(&ben, &got)?, ["Milk"]);
    assert!(Checklists::new(&ben).by_note(&got)?.is_none());
    Ok(())
}

/// What `to` opens of what `from` sends of `note`.
fn sent(from: &Store, to: &Store, note: &str) -> Made<crate::sent::Opened> {
    let item = from.sent_item_of(note, false)?;
    let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
    let file = crate::sent::encode(&item, &from.signing_key()?, &[], &mut random)?;
    let json: serde_json::Value = serde_json::from_slice(&file)?;
    assert_eq!(json["format"], crate::sent::LIST_FORMAT);
    Ok(to.open_sent_item(&file)?.map_err(|_| "a Sent Item")?)
}

#[test]
fn a_sent_shopping_list_carries_its_lines_and_they_arrive_open() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let ben = clock.device(2, "Ben")?;
    let note = shopping(&ann, "Shop\nMilk\nBread\nEggs", None)?;
    let lists = ShoppingLists::new(&ann);
    lists.set_reappear(
        &id_of(&ann, &note, "Eggs")?,
        Reappear::After(2, ReappearUnit::Week),
    )?;
    lists.set_reappear(&id_of(&ann, &note, "Bread")?, Reappear::Once)?;
    lists.check(&id_of(&ann, &note, "Bread")?, &wall)?;
    lists.check(&id_of(&ann, &note, "Milk")?, &wall)?;
    let opened = sent(&ann, &ben, &note)?;
    assert!(opened.item.shopping);
    let carried: Vec<String> = opened
        .item
        .lines
        .iter()
        .flatten()
        .map(|l| {
            format!(
                "{} {}",
                l.text,
                l.reappear.map(Reappear::code).unwrap_or_default()
            )
        })
        .collect();
    assert_eq!(carried, ["Eggs 2w", "Milk none"]);
    let got = ben
        .accept_sent_item(&opened.item, false)?
        .ok_or("accepted")?;
    assert_eq!(shown(&ben, &got)?, ["Eggs", "Milk"]);
    assert_eq!(
        line_of(&ben, &got, "Eggs")?.reappear,
        Reappear::After(2, ReappearUnit::Week)
    );
    assert!(Checklists::new(&ben).by_note(&got)?.is_none());
    assert!(Notes::new(&ben).by_id(&got)?.ok_or("note")?.is_unsorted());
    Ok(())
}

#[test]
fn an_update_changes_the_senders_lines_and_keeps_the_receivers() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let ben = clock.device(2, "Ben")?;
    let note = shopping(&ann, "Shop\nMilk\nEggs", None)?;
    let got = ben
        .accept_sent_item(&sent(&ann, &ben, &note)?.item, false)?
        .ok_or("accepted")?;
    let his = ShoppingLists::new(&ben);
    his.check(&id_of(&ben, &got, "Eggs")?, &wall)?;
    his.add(&got, "Coffee", &wall)?;
    let hers = ShoppingLists::new(&ann);
    hers.rename(&id_of(&ann, &note, "Eggs")?, "Free-range eggs")?;
    hers.delete(&id_of(&ann, &note, "Milk")?)?;
    hers.add(&note, "Tea", &wall)?;
    let opened = sent(&ann, &ben, &note)?;
    let changes = opened.changes.as_ref().ok_or("an update")?;
    assert!(!changes.makes_shopping_list);
    let added: Vec<&str> = changes
        .lines_added
        .iter()
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(added, ["Tea"]);
    assert_eq!(changes.lines_renamed.len(), 1);
    assert_eq!(changes.lines_renamed[0].after, "Free-range eggs");
    let removed: Vec<&str> = changes
        .lines_removed
        .iter()
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(removed, ["Milk"]);
    ben.accept_sent_item(&opened.item, false)?;
    assert_eq!(shown(&ben, &got)?, ["Coffee", "Tea", "~Free-range eggs"]);
    Ok(())
}

#[test]
fn a_line_gone_here_is_not_added_again() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let ben = clock.device(2, "Ben")?;
    let note = shopping(&ann, "Shop\nMilk", None)?;
    let got = ben
        .accept_sent_item(&sent(&ann, &ben, &note)?.item, false)?
        .ok_or("accepted")?;
    let milk = id_of(&ben, &got, "Milk")?;
    ShoppingLists::new(&ben).set_reappear(&milk, Reappear::Once)?;
    ShoppingLists::new(&ben).check(&milk, &wall)?;
    let opened = sent(&ann, &ben, &note)?;
    assert!(opened.changes.as_ref().ok_or("an update")?.is_empty());
    Ok(())
}

#[test]
fn a_note_follows_the_sender_into_a_shopping_list_and_out() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let ben = clock.device(2, "Ben")?;
    let note = Notes::new(&ann).capture("Shop\nMilk", None, None)?.id;
    let got = ben
        .accept_sent_item(&sent_plain(&ann, &ben, &note)?, false)?
        .ok_or("accepted")?;
    ShoppingLists::new(&ann).make(&note)?;
    let made = sent(&ann, &ben, &note)?;
    let changes = made.changes.as_ref().ok_or("an update")?;
    assert!(changes.makes_shopping_list && !changes.makes_checklist);
    ben.accept_sent_item(&made.item, false)?;
    assert_eq!(shown(&ben, &got)?, ["Milk"]);
    ShoppingLists::new(&ann).to_plain(&note, &wall)?;
    let plain = ann.sent_item_of(&note, false)?;
    let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
    let file = crate::sent::encode(&plain, &ann.signing_key()?, &[], &mut random)?;
    let opened = ben.open_sent_item(&file)?.map_err(|_| "a Sent Item")?;
    assert!(opened.changes.as_ref().ok_or("an update")?.makes_plain);
    ben.accept_sent_item(&opened.item, false)?;
    assert!(!ShoppingLists::new(&ben).is_shopping_list(&got)?);
    Ok(())
}

/// What `to` opens of the plain Note `note` that `from` sends.
fn sent_plain(from: &Store, to: &Store, note: &str) -> Made<crate::sent::SentItem> {
    let item = from.sent_item_of(note, false)?;
    let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
    let file = crate::sent::encode(&item, &from.signing_key()?, &[], &mut random)?;
    Ok(to.open_sent_item(&file)?.map_err(|_| "a Sent Item")?.item)
}

#[test]
fn a_file_naming_another_list_is_no_sent_item() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = shopping(&ann, "Shop\nMilk", None)?;
    let item = ann.sent_item_of(&note, false)?;
    let payload = item.encode();
    assert!(crate::sent::SentItem::parse(&payload).is_some_and(|i| i.shopping));
    let other = payload.replacen("\"list\":\"shopping\"", "\"list\":\"pantry\"", 1);
    assert_ne!(other, payload);
    assert!(crate::sent::SentItem::parse(&other).is_none());
    Ok(())
}

/// Writes how this Device's Agenda showed Shopping Lists as a version
/// before #186 did, which no app offers any more.
fn old_setting(store: &Store, way: ShoppingAgenda) -> Made<()> {
    Ok(store.set_local_setting("shoppingAgenda", way.code())?)
}

/// Today's Agenda rows of the Shopping Lists of `store`: "title: n
/// open" for one row with the count, "title: Line" for each open Line.
fn agenda_rows(store: &Store) -> Made<Vec<String>> {
    let notes = Notes::new(store);
    let mut out = Vec::new();
    for row in ShoppingLists::new(store).agenda_rows(&wall)? {
        let title = notes.by_id(&row.list.note_id)?.ok_or("the Note")?.text;
        match row.shows {
            ShoppingOnAgenda::Count => out.push(format!("{title}: {} open", row.lines.len())),
            ShoppingOnAgenda::Lines => {
                for line in &row.lines {
                    out.push(format!("{title}: {}", line.text));
                }
            }
            ShoppingOnAgenda::Off => return Err("an Off row".into()),
        }
    }
    Ok(out)
}

/// Every value of a Shopping List's Agenda choice `store` holds, with
/// the Device that wrote it.
fn choices(store: &Store) -> Made<Vec<(String, Option<String>)>> {
    Ok(store
        .all_entries()?
        .into_iter()
        .filter(|e| e.entity.starts_with(checklist_keys::AGENDA_PREFIX) && e.field == keys::AGENDA)
        .map(|e| (e.device, e.value))
        .collect())
}

#[test]
fn asked_for_a_line_checked_today_stays_after_the_open_ones_for_the_day_struck_or_gone()
-> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let lists = ShoppingLists::new(&ann);
    let week = shopping(&ann, "Groceries\nMilk\nEggs\nTea", None)?;
    lists.set_on_agenda(&week, ShoppingOnAgenda::Lines)?;
    let eggs = id_of(&ann, &week, "Eggs")?;
    let milk = id_of(&ann, &week, "Milk")?;
    lists.set_reappear(&eggs, Reappear::Once)?;
    lists.set_reappear(&milk, Reappear::After(1, ReappearUnit::Day))?;
    lists.check(&eggs, &wall)?;
    lists.check(&milk, &wall)?;
    let today = || -> Made<Vec<String>> {
        let mut out = Vec::new();
        for row in lists.agenda_rows_with(&wall, true)? {
            let open = row.list.open();
            for line in &row.lines {
                let struck = if open.contains(&line) { "" } else { " x" };
                out.push(format!("{}{struck}", line.text));
            }
        }
        Ok(out)
    };
    assert_eq!(agenda_rows(&ann)?, ["Groceries: Tea"], "unasked");
    assert_eq!(today()?, ["Tea", "Eggs x", "Milk x"]);
    lists.check(&id_of(&ann, &week, "Tea")?, &wall)?;
    assert_eq!(today()?, ["Eggs x", "Milk x", "Tea x"], "none open");
    // Gone Once, Eggs comes back by its box as by its name.
    lists.uncheck(&eggs, &wall)?;
    assert_eq!(today()?, ["Eggs", "Milk x", "Tea x"]);
    lists.uncheck(&eggs, &wall)?;
    assert_eq!(today()?, ["Eggs", "Milk x", "Tea x"], "open stays");
    // Past midnight on the wall clock only Eggs is left; Milk is back.
    clock.set(utc(5, 23, 30, 0)?);
    assert_eq!(today()?, ["Milk", "Eggs"]);
    // By when each was checked, not by its id.
    let hardware = shopping(&ann, "Hardware\nNails\nGlue\nTape", None)?;
    let mut ids: Vec<String> = ShoppingLists::new(&ann)
        .by_note(&hardware, &wall)?
        .ok_or("a Shopping List")?
        .lines
        .into_iter()
        .map(|l| l.id)
        .collect();
    ids.sort_by(|a, b| utf16_cmp(b, a));
    for id in &ids {
        lists.check(id, &wall)?;
    }
    let rows = lists.agenda_rows_with(&wall, true)?;
    let row = rows
        .iter()
        .find(|r| r.list.note_id == hardware)
        .ok_or("its row")?;
    let shown: Vec<&String> = row.lines.iter().map(|l| &l.id).collect();
    assert_eq!(shown, ids.iter().collect::<Vec<_>>());
    Ok(())
}

#[test]
fn each_list_shows_on_the_agenda_as_chosen_for_it() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let lists = ShoppingLists::new(&ann);
    let week = shopping(&ann, "Groceries\nMilk\nEggs\nTea", None)?;
    let hardware = shopping(&ann, "Hardware\nNails\nGlue", None)?;
    let pharmacy = shopping(&ann, "Pharmacy\nPlasters", None)?;
    lists.check(&id_of(&ann, &week, "Tea")?, &wall)?;
    assert_eq!(lists.on_agenda(&week)?, ShoppingOnAgenda::Count);
    assert!(lists.may_show_on_agenda(&week)?);
    assert_eq!(
        agenda_rows(&ann)?,
        ["Groceries: 2 open", "Hardware: 2 open", "Pharmacy: 1 open"]
    );
    lists.set_on_agenda(&hardware, ShoppingOnAgenda::Lines)?;
    lists.set_on_agenda(&pharmacy, ShoppingOnAgenda::Off)?;
    assert_eq!(lists.on_agenda(&pharmacy)?, ShoppingOnAgenda::Off);
    assert_eq!(
        agenda_rows(&ann)?,
        ["Groceries: 2 open", "Hardware: Nails", "Hardware: Glue"]
    );
    let count = ann.all_entries()?.len();
    lists.set_on_agenda(&pharmacy, ShoppingOnAgenda::Off)?;
    assert_eq!(ann.all_entries()?.len(), count, "the same again");
    // Chosen as the default shows it, it is written all the same.
    lists.set_on_agenda(&week, ShoppingOnAgenda::Count)?;
    let values: Vec<Option<String>> = choices(&ann)?.into_iter().map(|(_, v)| v).collect();
    assert_eq!(
        values,
        [
            Some("lines".into()),
            Some("off".into()),
            Some("count".into())
        ]
    );
    let count = ann.all_entries()?.len();
    lists.agenda_rows(&wall)?;
    lists.on_agenda(&week)?;
    assert_eq!(ann.all_entries()?.len(), count, "reading writes nothing");
    Ok(())
}

#[test]
fn a_list_with_no_open_line_never_shows() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let lists = ShoppingLists::new(&ann);
    let week = shopping(&ann, "Groceries\nMilk\nEggs", None)?;
    let hardware = shopping(&ann, "Hardware\nNails", None)?;
    weekly(&ann, &week)?;
    weekly(&ann, &hardware)?;
    shopping(&ann, "Title only", None)?;
    lists.set_on_agenda(&week, ShoppingOnAgenda::Lines)?;
    lists.set_reappear(&id_of(&ann, &week, "Eggs")?, Reappear::Once)?;
    for text in ["Milk", "Eggs"] {
        lists.check(&id_of(&ann, &week, text)?, &wall)?;
    }
    lists.check(&id_of(&ann, &hardware, "Nails")?, &wall)?;
    assert!(agenda_rows(&ann)?.is_empty());
    // A week on, Milk and Nails are back, Eggs gone Once.
    clock.set(utc(12, 9, 0, 0)?);
    assert_eq!(agenda_rows(&ann)?, ["Groceries: Milk", "Hardware: 1 open"]);
    Ok(())
}

#[test]
fn a_list_nobody_chose_for_shows_as_the_device_showed_shopping_lists() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let lists = ShoppingLists::new(&ann);
    let week = shopping(&ann, "Groceries\nMilk", None)?;
    let chosen = shopping(&ann, "Hardware\nNails", None)?;
    lists.set_on_agenda(&chosen, ShoppingOnAgenda::Count)?;
    for (old, shows) in [
        (ShoppingAgenda::Off, ShoppingOnAgenda::Off),
        (ShoppingAgenda::Lines, ShoppingOnAgenda::Lines),
        (ShoppingAgenda::Back, ShoppingOnAgenda::Count),
        (ShoppingAgenda::Open, ShoppingOnAgenda::Count),
    ] {
        old_setting(&ann, old)?;
        assert_eq!(lists.on_agenda(&week)?, shows, "{old:?}");
        assert_eq!(
            lists.on_agenda(&chosen)?,
            ShoppingOnAgenda::Count,
            "{old:?}"
        );
    }
    old_setting(&ann, ShoppingAgenda::Off)?;
    assert_eq!(agenda_rows(&ann)?, ["Hardware: 1 open"]);
    ann.set_local_setting("shoppingAgenda", "sideways")?;
    assert_eq!(lists.on_agenda(&week)?, ShoppingOnAgenda::Count);
    Ok(())
}

#[test]
fn an_agenda_choice_no_app_writes_reads_as_one_row_with_the_count() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    old_setting(&ann, ShoppingAgenda::Off)?;
    let week = shopping(&ann, "Groceries\nMilk", None)?;
    let entity = format!("{}{week}", checklist_keys::AGENDA_PREFIX);
    for value in ["OFF", "off ", "later", ""] {
        ann.append(&entity, keys::AGENDA, Some(value), None)?;
        assert_eq!(
            ShoppingLists::new(&ann).on_agenda(&week)?,
            ShoppingOnAgenda::Count,
            "{value}"
        );
    }
    // Null is no choice: the default again.
    ann.append(&entity, keys::AGENDA, None, None)?;
    assert_eq!(
        ShoppingLists::new(&ann).on_agenda(&week)?,
        ShoppingOnAgenda::Off
    );
    Ok(())
}

#[test]
fn a_checklists_switch_is_not_a_shopping_lists_choice() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let note = Notes::new(&ann).capture("Groceries\nMilk", None, None)?.id;
    let checklists = Checklists::new(&ann);
    checklists.make(&note)?;
    checklists.set_on_agenda(&note, ListOnAgenda::Off)?;
    checklists.to_plain(&note)?;
    let lists = ShoppingLists::new(&ann);
    lists.make(&note)?;
    assert_eq!(lists.on_agenda(&note)?, ShoppingOnAgenda::Count);
    assert_eq!(agenda_rows(&ann)?, ["Groceries: 1 open"]);
    lists.set_on_agenda(&note, ShoppingOnAgenda::Off)?;
    lists.to_plain(&note, &wall)?;
    checklists.make(&note)?;
    checklists.set_on_agenda(&note, ListOnAgenda::Lines)?;
    assert_eq!(checklists.agenda_rows()?.len(), 1);
    Ok(())
}

#[test]
fn only_shopping_lists_take_an_agenda_choice() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let lists = ShoppingLists::new(&ann);
    let plain = notes.capture("Plain\nline", None, None)?.id;
    let trip = notes.capture("Trip\nTent", None, None)?.id;
    Checklists::new(&ann).make(&trip)?;
    let trashed = shopping(&ann, "Trashed\nOne", None)?;
    notes.trash(&trashed)?;
    let merged = shopping(&ann, "Merged\nOne", None)?;
    let into = notes.capture("Into", None, None)?.id;
    notes.merge(&merged, &into)?;
    let errands = shopping(&ann, "Errands\nStamps", None)?;
    Routines::new(&ann).spawn(
        &errands,
        &Schedule::new(Day::new(2026, 3, 5), &[], None).ok_or("a schedule")?,
    )?;
    for id in [&plain, &trip, &trashed, &merged, &into] {
        assert!(!lists.may_show_on_agenda(id)?, "{id}");
    }
    assert!(lists.may_show_on_agenda(&errands)?);
    assert_eq!(agenda_rows(&ann)?, ["Errands: 1 open"]);
    assert!(lists.set_on_agenda(&plain, ShoppingOnAgenda::Off).is_err());
    assert!(lists.set_on_agenda(&trip, ShoppingOnAgenda::Off).is_err());
    assert!(choices(&ann)?.is_empty());
    Ok(())
}

#[test]
fn in_a_shared_area_each_person_chooses_for_their_own_agenda() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let ben = clock.device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    invite(&ann, &ben, &area)?;
    let note = shopping(&ann, "Shop\nMilk\nEggs", Some(&area))?;
    exchange(&ben, &ann)?;
    // Ben, a Member, may choose for a list he did not write.
    ShoppingLists::new(&ben).set_on_agenda(&note, ShoppingOnAgenda::Lines)?;
    ShoppingLists::new(&ann).set_on_agenda(&note, ShoppingOnAgenda::Off)?;
    exchange(&ben, &ann)?;
    exchange(&ben, &ann)?;
    assert!(agenda_rows(&ann)?.is_empty());
    assert_eq!(agenda_rows(&ben)?, ["Shop: Milk", "Shop: Eggs"]);
    assert_eq!(
        choices(&ann)?,
        [(ann.device_id()?, Some("off".to_string()))]
    );
    assert_eq!(
        choices(&ben)?,
        [(ben.device_id()?, Some("lines".to_string()))]
    );
    Ok(())
}

#[test]
fn ones_own_devices_carry_the_agenda_choice() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let desk = clock.device(3, "Ann at the desk")?;
    own_exchange(&desk, &ann)?;
    let note = shopping(&ann, "Shop\nMilk", None)?;
    own_exchange(&desk, &ann)?;
    // The desk's old setting showed nothing; her choice shows it there.
    old_setting(&desk, ShoppingAgenda::Off)?;
    assert!(agenda_rows(&desk)?.is_empty());
    ShoppingLists::new(&ann).set_on_agenda(&note, ShoppingOnAgenda::Lines)?;
    own_exchange(&desk, &ann)?;
    assert_eq!(agenda_rows(&desk)?, ["Shop: Milk"]);
    ShoppingLists::new(&desk).set_on_agenda(&note, ShoppingOnAgenda::Count)?;
    send(&desk, &ann)?;
    assert_eq!(agenda_rows(&ann)?, ["Shop: 1 open"]);
    Ok(())
}

#[test]
fn an_agenda_choice_another_persons_device_wrote_counts_for_nothing() -> TestResult {
    let clock = Clock::new()?;
    let ann = clock.device(1, "Ann")?;
    let ben = clock.device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    invite(&ann, &ben, &area)?;
    let note = shopping(&ann, "Shop\nMilk", Some(&area))?;
    exchange(&ben, &ann)?;
    // As a modified app would: Ben takes the list off Ann's Agenda.
    let forged = ben.append(
        &format!("{}{note}", checklist_keys::AGENDA_PREFIX),
        keys::AGENDA,
        Some("off"),
        None,
    )?;
    ann.apply_entries(
        &[forged.wire()],
        &[],
        Some(&ben.device_id()?),
        &mut crate::signing::ImportReport::default(),
    )?;
    assert_eq!(
        choices(&ann)?,
        [(ben.device_id()?, Some("off".to_string()))]
    );
    let lists = ShoppingLists::new(&ann);
    assert_eq!(lists.on_agenda(&note)?, ShoppingOnAgenda::Count);
    assert_eq!(agenda_rows(&ann)?, ["Shop: 1 open"]);
    // Nor does it count as her own choice made already.
    lists.set_on_agenda(&note, ShoppingOnAgenda::Off)?;
    assert!(agenda_rows(&ann)?.is_empty());
    Ok(())
}
