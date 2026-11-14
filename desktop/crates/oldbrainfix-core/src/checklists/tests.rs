use super::*;
use crate::lan::{
    Joining, answer_sync, answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
};
use crate::shared::{AreaRole, Invite};
use crate::signing::{ImportReport, SigningKey};
use crate::store::tests::ticking;
use crate::sync::wire;
use crate::{Entry, OwnProfile, Places, Search};
use chrono::Duration;

pub(crate) type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One clock for every Device of a test, a second further at each
/// reading, so what was written later wins everywhere.
static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

fn clocked(seed: u8) -> Made<Store> {
    let start = ticking()();
    let clock: crate::Clock = Box::new(move || {
        let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        start + Duration::seconds(n)
    });
    Ok(Store::in_memory(clock, move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?)
}

fn device(seed: u8, name: &str) -> Made<Store> {
    let store = clocked(seed)?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

/// One LAN session, `joiner` asking `host`, between two people.
pub(crate) fn exchange(joiner: &Store, host: &Store) -> TestResult {
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

/// Everything `from` holds that `to` lacks, taken in: two of the
/// user's own Devices.
pub(crate) fn send(from: &Store, to: &Store) -> TestResult {
    to.apply_entries(
        &wire(&from.entries_since(&to.version_vector()?)?),
        &from.key_records()?,
        None,
        &mut ImportReport::default(),
    )?;
    Ok(())
}

/// `member` invited by `ann` into `area` as a Member.
pub(crate) fn invite(ann: &Store, member: &Store, area: &str) -> TestResult {
    let invite = Invite {
        area: area.to_string(),
        role: AreaRole::Member,
    };
    let request = sync_request_as(
        member,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let (answer, _) =
        answer_sync_inviting(ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(member, &answer, true, None)?;
    for (store, peer) in [(ann, member), (member, ann)] {
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
    Ok(())
}

/// Ann's Household shared with Ben as a Member.
fn household() -> Made<(Store, Store, String)> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    invite(&ann, &ben, &area)?;
    Ok((ann, ben, area))
}

/// A Checklist made from a new Note with `text`.
fn checklist(store: &Store, text: &str, place: Option<&str>) -> Made<String> {
    let id = Notes::new(store).capture(text, place, None)?.id;
    Checklists::new(store).make(&id)?;
    Ok(id)
}

/// The Lines of the Checklist of `note` as `text`, `  text` for one of
/// a group, `[x]` for one shown checked.
fn shown(store: &Store, note: &str) -> Made<Vec<String>> {
    let list = Checklists::new(store).by_note(note)?;
    Ok(list
        .map(|l| l.lines)
        .unwrap_or_default()
        .iter()
        .map(|l| {
            format!(
                "{}{}{}",
                if l.in_group() { "  " } else { "" },
                if l.checked { "[x] " } else { "" },
                l.text
            )
        })
        .collect())
}

fn id_of(store: &Store, note: &str, text: &str) -> Made<String> {
    let list = Checklists::new(store).by_note(note)?.ok_or("a Checklist")?;
    Ok(list
        .lines
        .iter()
        .find(|l| l.text == text)
        .ok_or("a Line")?
        .id
        .clone())
}

#[test]
fn the_first_line_stays_and_an_indented_one_goes_under_the_line_above() -> TestResult {
    let ann = device(1, "Ann")?;
    let note = Notes::new(&ann).capture(
        "Packing\r\nSocks\n  Blue\n\tRed\n\n✓ Passport\r   \nTent\n  ✓ Pegs",
        None,
        None,
    )?;
    let lists = Checklists::new(&ann);
    assert!(lists.may_make(&note.id)?);
    lists.make(&note.id)?;
    let made = Notes::new(&ann).by_id(&note.id)?.ok_or("the Note")?;
    assert_eq!(made.text, "Packing");
    assert!(!made.is_unsorted());
    assert_eq!(
        shown(&ann, &note.id)?,
        [
            "Socks",
            "  Blue",
            "  Red",
            "[x] Passport",
            "[x] Tent",
            "  [x] Pegs"
        ]
    );
    let list = lists.by_note(&note.id)?.ok_or("a Checklist")?;
    assert_eq!((list.checked_count(), list.count()), (3, 6));
    assert!(!lists.may_make(&note.id)?);
    assert!(lists.make(&note.id).is_err());
    // Two levels only; a first Line indented stays at the top.
    let deep = checklist(&ann, "List\n  a\n    b\n      c\nd", None)?;
    assert_eq!(shown(&ann, &deep)?, ["a", "  b", "  c", "d"]);
    Ok(())
}

#[test]
fn from_a_formatted_text_the_marks_of_items_and_quotes_go() -> TestResult {
    let ann = device(1, "Ann")?;
    let note = Notes::new(&ann).capture_as(
        "# Packing\n- Socks\n  - Blue\n* Tent\n\t+ ✓ Pegs\n1. Map\n\
         > Torch\n- \n**Rope**\n## Later\n-Knife",
        text_format::MARKDOWN,
        None,
        None,
    )?;
    Checklists::new(&ann).make(&note.id)?;
    let made = Notes::new(&ann).by_id(&note.id)?.ok_or("the Note")?;
    assert_eq!(made.text, "# Packing");
    assert_eq!(
        shown(&ann, &note.id)?,
        [
            "Socks",
            "  Blue",
            "[x] Tent",
            "  [x] Pegs",
            "Map",
            "Torch",
            "**Rope**",
            "## Later",
            "-Knife"
        ]
    );
    // A plain text keeps them as written.
    let plain = checklist(&ann, "Packing\n- Socks\n> Torch", None)?;
    assert_eq!(shown(&ann, &plain)?, ["- Socks", "> Torch"]);
    Ok(())
}

#[test]
fn a_note_is_at_most_one_of_a_tally_and_a_checklist() -> TestResult {
    let ann = device(1, "Ann")?;
    let gym = Notes::new(&ann).capture("Gym\nPush-ups", None, None)?.id;
    Tallies::new(&ann).add_field(&gym, "Push-ups", "", None, None)?;
    assert!(!Checklists::new(&ann).may_make(&gym)?);
    assert!(Checklists::new(&ann).make(&gym).is_err());
    let list = checklist(&ann, "Packing\nSocks", None)?;
    assert!(
        Tallies::new(&ann)
            .add_field(&list, "Socks", "", None, None)
            .is_err()
    );
    assert!(!Tallies::new(&ann).is_tally(&list)?);
    Ok(())
}

#[test]
fn kinds_and_places_no_app_writes_are_read_without_fail() -> TestResult {
    let ann = device(1, "Ann")?;
    let note = Notes::new(&ann).capture("Odd\nOne", None, None)?.id;
    for kind in [
        "checklist",
        "checklist:",
        ":x",
        "checklist:a:b",
        "shoppingList:x",
    ] {
        ann.append(&note, keys::KIND, Some(kind), None)?;
        assert!(Checklists::new(&ann).by_note(&note)?.is_none(), "{kind}");
    }
    ann.append(&note, keys::KIND, None, None)?;
    let lists = Checklists::new(&ann);
    lists.make(&note)?;
    let making = lists.by_note(&note)?.ok_or("a Checklist")?.making;
    for (i, order) in [Some("V0"), Some(""), Some("é"), None, Some("V")]
        .iter()
        .enumerate()
    {
        let id = format!("line:odd{i}");
        ann.append(&id, keys::TYPE, Some(keys::TYPE_LINE), None)?;
        ann.append(&id, keys::NOTE, Some(&note), None)?;
        ann.append(&id, keys::MADE, Some(&making), None)?;
        ann.append(&id, keys::TEXT, Some(&format!("Odd {i}")), None)?;
        if let Some(order) = order {
            ann.append(&id, keys::ORDER, Some(order), None)?;
        }
    }
    assert_eq!(
        shown(&ann, &note)?,
        ["One", "Odd 4", "Odd 1", "Odd 3", "Odd 0", "Odd 2"]
    );
    let added = lists.add(&note, "Added", Some(3), false)?;
    lists.move_line("line:odd2", Some("line:odd4"))?;
    lists.move_line(&added, None)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["One", "Odd 4", "Odd 2", "Added", "Odd 1", "Odd 3", "Odd 0"]
    );
    Ok(())
}

#[test]
fn a_parent_and_its_group_check_each_other() -> TestResult {
    let ann = device(1, "Ann")?;
    let note = checklist(&ann, "Trip\nBath\n  Soap\n  Towel\nTent", None)?;
    let lists = Checklists::new(&ann);
    let text = ann.current(&note, note_keys::TEXT)?;
    lists.check(&id_of(&ann, &note, "Soap")?)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["Bath", "  [x] Soap", "  Towel", "Tent"]
    );
    let mark = ann.current_seq()?;
    lists.check(&id_of(&ann, &note, "Towel")?)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["[x] Bath", "  [x] Soap", "  [x] Towel", "Tent"]
    );
    let written: Vec<String> = ann
        .entries_after(mark)?
        .into_iter()
        .map(|e| e.entity)
        .collect();
    assert_eq!(
        written,
        [id_of(&ann, &note, "Towel")?, id_of(&ann, &note, "Bath")?]
    );
    lists.uncheck(&id_of(&ann, &note, "Soap")?)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["Bath", "  Soap", "  [x] Towel", "Tent"]
    );
    lists.check(&id_of(&ann, &note, "Bath")?)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["[x] Bath", "  [x] Soap", "  [x] Towel", "Tent"]
    );
    lists.uncheck(&id_of(&ann, &note, "Bath")?)?;
    assert_eq!(shown(&ann, &note)?, ["Bath", "  Soap", "  Towel", "Tent"]);
    assert_eq!(ann.current(&note, note_keys::TEXT)?, text);
    let shop = checklist(&ann, "Shop\n✓ Milk\n✓ Eggs\nBread\n  ✓ Rye", None)?;
    lists.uncheck_all(&shop)?;
    assert_eq!(shown(&ann, &shop)?, ["Milk", "Eggs", "Bread", "  Rye"]);
    Ok(())
}

#[test]
fn lines_are_added_renamed_deleted_restored_moved_and_indented() -> TestResult {
    let ann = device(1, "Ann")?;
    let lists = Checklists::new(&ann);
    let note = checklist(&ann, "Shop\nMilk\nEggs", None)?;
    lists.add(&note, "  Bread  ", None, false)?;
    lists.add(&note, "Butter", Some(0), false)?;
    lists.add(&note, "Oat", Some(2), true)?;
    lists.add(&note, "Top", Some(0), true)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["Top", "Butter", "Milk", "  Oat", "Eggs", "Bread"]
    );
    assert!(lists.add(&note, " \n ", None, false).is_err());
    let milk = id_of(&ann, &note, "Milk")?;
    lists.rename(&milk, "Whole\nmilk")?;
    lists.delete(&milk)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["Top", "Butter", "Oat", "Eggs", "Bread"]
    );
    lists.restore(&milk)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["Top", "Butter", "Whole milk", "Oat", "Eggs", "Bread"]
    );
    assert!(lists.restore(&milk).is_err());

    let trip = checklist(&ann, "Trip\nBath\n  Soap\n  Towel\nTent\nMap", None)?;
    let line = |text: &str| id_of(&ann, &trip, text);
    lists.move_line(&line("Map")?, Some(&line("Towel")?))?;
    assert_eq!(
        shown(&ann, &trip)?,
        ["Bath", "  Soap", "  Map", "  Towel", "Tent"]
    );
    lists.move_line(&line("Map")?, Some(&line("Bath")?))?;
    assert_eq!(
        shown(&ann, &trip)?,
        ["Map", "Bath", "  Soap", "  Towel", "Tent"]
    );
    lists.move_line(&line("Bath")?, None)?;
    assert_eq!(
        shown(&ann, &trip)?,
        ["Map", "Tent", "Bath", "  Soap", "  Towel"]
    );
    lists.indent(&line("Tent")?, true)?;
    lists.move_line(&line("Bath")?, Some(&line("Tent")?))?;
    assert_eq!(
        shown(&ann, &trip)?,
        ["Map", "  Tent", "Bath", "  Soap", "  Towel"]
    );
    lists.indent(&line("Soap")?, false)?;
    assert_eq!(
        shown(&ann, &trip)?,
        ["Map", "  Tent", "Bath", "Soap", "  Towel"]
    );
    lists.indent(&line("Soap")?, true)?;
    assert!(lists.indent(&line("Map")?, true).is_err());
    lists.delete(&line("Bath")?)?;
    assert_eq!(shown(&ann, &trip)?, ["Map", "  Tent", "Soap", "Towel"]);
    Ok(())
}

#[test]
fn the_deleted_lines_are_listed_the_last_deleted_first_while_a_restore_brings_them_back()
-> TestResult {
    let ann = device(1, "Ann")?;
    let lists = Checklists::new(&ann);
    let note = checklist(&ann, "Shop\nMilk\nEggs\nTea", None)?;
    assert!(lists.deleted_lines(&note)?.is_empty());
    let milk = id_of(&ann, &note, "Milk")?;
    let tea = id_of(&ann, &note, "Tea")?;
    lists.delete(&milk)?;
    lists.delete(&tea)?;
    let listed = |lists: Vec<DeletedLine>| -> Vec<(String, String)> {
        lists.into_iter().map(|l| (l.id, l.text)).collect()
    };
    assert_eq!(
        listed(lists.deleted_lines(&note)?),
        [
            (tea.clone(), "Tea".to_string()),
            (milk.clone(), "Milk".to_string())
        ]
    );
    lists.restore(&tea)?;
    assert_eq!(
        listed(lists.deleted_lines(&note)?),
        [(milk, "Milk".to_string())]
    );
    // Another list's Lines are not its own.
    assert!(
        crate::ShoppingLists::new(&ann)
            .deleted_lines(&note)?
            .is_empty()
    );
    // Made anew, the Lines of the old making are gone for good.
    lists.to_plain(&note)?;
    assert!(lists.deleted_lines(&note)?.is_empty());
    lists.make(&note)?;
    assert!(lists.deleted_lines(&note)?.is_empty());
    Ok(())
}

#[test]
fn keys_fit_between_any_two_and_stay_short_at_the_end() {
    assert_eq!(order::place(None, None), "V");
    assert_eq!(order::after("V"), "W");
    assert_eq!(order::after("Vz"), "W");
    assert_eq!(order::after("z"), "zV");
    assert_eq!(order::after("zzz"), "zzzV");
    assert_eq!(order::between("", "V"), "F");
    assert_eq!(order::between("", "1"), "0V");
    assert_eq!(order::between("", "0V"), "0F");
    assert_eq!(order::between("V", "W"), "VV");
    assert_eq!(order::between("V", "V05"), "V02");
    assert_eq!(order::between("V", "X"), "W");
    assert_eq!(order::between("VV", "W"), "VW");
    assert_eq!(order::between("Vz", "W"), "VzV");
    // No key slices a character apart.
    assert_eq!(order::between("é", "éa"), "éV");
    assert_eq!(order::after("é"), "éV");
    let (mut low, mut high) = ("V".to_string(), "W".to_string());
    for i in 0..200 {
        let mid = order::between(&low, &high);
        assert!(order::is_valid(Some(&mid)), "{mid}");
        assert!(low < mid && mid < high);
        if i % 2 == 0 {
            low = mid;
        } else {
            high = mid;
        }
    }
    let mut key = String::new();
    for _ in 0..100 {
        key = order::after(&key);
    }
    assert!(key.len() <= 4);
}

#[test]
fn back_to_a_plain_note_and_made_again_afresh() -> TestResult {
    let ann = device(1, "Ann")?;
    let lists = Checklists::new(&ann);
    let note = checklist(&ann, "Trip\nBath\n  Soap\n  ✓ Towel\n✓ Tent", None)?;
    let soap = id_of(&ann, &note, "Soap")?;
    lists.check(&soap)?;
    lists.add(&note, "Map", None, false)?;
    lists.to_plain(&note)?;
    assert!(lists.by_note(&note)?.is_none());
    let text = Notes::new(&ann).by_id(&note)?.ok_or("the Note")?.text;
    assert_eq!(text, "Trip\n✓ Bath\n  ✓ Soap\n  ✓ Towel\n✓ Tent\nMap");
    assert!(ann.current(&soap, keys::CHECKED)?.is_some());
    lists.make(&note)?;
    assert_eq!(
        shown(&ann, &note)?,
        ["[x] Bath", "  [x] Soap", "  [x] Towel", "[x] Tent", "Map"]
    );
    assert!(lists.by_note(&note)?.ok_or("a list")?.line(&soap).is_none());
    let found: Vec<String> = Search::new(&ann)
        .notes("SOAP", None)?
        .into_iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(found, std::slice::from_ref(&note));
    lists.delete(&id_of(&ann, &note, "Soap")?)?;
    assert!(Search::new(&ann).notes("soap", None)?.is_empty());
    Ok(())
}

#[test]
fn every_member_checks_unchecks_and_adds_lines() -> TestResult {
    let (ann, ben, area) = household()?;
    let note = checklist(&ann, "Shop\nMilk\nEggs", Some(&area))?;
    exchange(&ben, &ann)?;
    let lists = Checklists::new(&ben);
    assert!(lists.may_check(&note)?);
    lists.check(&id_of(&ben, &note, "Milk")?)?;
    lists.check(&id_of(&ben, &note, "Eggs")?)?;
    lists.uncheck(&id_of(&ben, &note, "Eggs")?)?;
    let bread = lists.add(&note, "Bread", None, false)?;
    exchange(&ben, &ann)?;
    assert_eq!(shown(&ann, &note)?, ["[x] Milk", "Eggs", "Bread"]);
    assert!(lists.may_edit(&bread)?);
    assert!(!lists.may_edit(&id_of(&ben, &note, "Milk")?)?);
    lists.rename(&bread, "Rye bread")?;
    exchange(&ben, &ann)?;
    Checklists::new(&ann).rename(&bread, "Brown bread")?;
    exchange(&ben, &ann)?;
    assert_eq!(shown(&ben, &note)?, ["[x] Milk", "Eggs", "Brown bread"]);
    Ok(())
}

#[test]
fn renaming_deleting_moving_and_the_kind_stay_with_the_author_and_the_managers() -> TestResult {
    let (ann, ben, area) = household()?;
    let note = checklist(&ann, "Shop\nMilk\nEggs\nTea", Some(&area))?;
    exchange(&ben, &ann)?;
    let lists = Checklists::new(&ben);
    let milk = id_of(&ben, &note, "Milk")?;
    let eggs = id_of(&ben, &note, "Eggs")?;
    assert!(lists.rename(&milk, "Beer").is_err());
    assert!(lists.delete(&milk).is_err());
    assert!(lists.move_line(&milk, None).is_err());
    assert!(lists.indent(&eggs, true).is_err());
    assert!(lists.to_plain(&note).is_err());
    // As a modified app would.
    let beer = StoredText::plain("Beer").encode();
    let tea = id_of(&ben, &note, "Tea")?;
    let forged = [
        ben.append(&milk, keys::TEXT, Some(&beer), None)?,
        ben.append(&eggs, keys::TRASHED, Some("2026-03-01T09:00:00.000Z"), None)?,
        ben.append(&milk, keys::ORDER, Some("z"), None)?,
        ben.append(&tea, keys::INDENT, Some("true"), None)?,
        ben.append(&note, keys::KIND, None, None)?,
    ];
    exchange(&ben, &ann)?;
    let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
    assert!(forged.iter().all(|e| held.contains(&e.id())));
    assert_eq!(shown(&ann, &note)?, ["Milk", "Eggs", "Tea"]);
    Ok(())
}

#[test]
fn a_member_who_made_the_checklist_changes_every_line_of_it() -> TestResult {
    let (ann, ben, area) = household()?;
    let cleo = device(3, "Cleo")?;
    invite(&ann, &cleo, &area)?;
    let note = checklist(&ben, "Shop\nMilk", Some(&area))?;
    exchange(&ben, &ann)?;
    exchange(&cleo, &ann)?;
    let tea = Checklists::new(&cleo).add(&note, "Tea", None, false)?;
    exchange(&cleo, &ann)?;
    exchange(&ben, &ann)?;
    assert!(Checklists::new(&ben).may_edit(&tea)?);
    Checklists::new(&ben).rename(&tea, "Green tea")?;
    Checklists::new(&ben).move_line(&tea, Some(&id_of(&ben, &note, "Milk")?))?;
    exchange(&ben, &ann)?;
    exchange(&cleo, &ann)?;
    assert_eq!(shown(&ann, &note)?, ["Green tea", "Milk"]);
    assert_eq!(shown(&cleo, &note)?, ["Green tea", "Milk"]);
    assert!(!Checklists::new(&cleo).may_edit(&id_of(&cleo, &note, "Milk")?)?);
    Ok(())
}

#[test]
fn a_checklist_made_before_its_note_was_filed_in_goes_along() -> TestResult {
    let (ann, ben, area) = household()?;
    let note = checklist(&ann, "Shop\nMilk", None)?;
    // Written in the Area since, so Ben's vector passes the Lines.
    Notes::new(&ann).capture("Bread", Some(&area), None)?;
    exchange(&ben, &ann)?;
    Notes::new(&ann).file(&note, &area)?;
    exchange(&ben, &ann)?;
    assert_eq!(shown(&ben, &note)?, ["Milk"]);
    Ok(())
}

#[test]
fn two_renames_of_one_line_are_a_conflict_and_a_later_check_wins() -> TestResult {
    let phone = clocked(1)?;
    let desk = clocked(2)?;
    let note = checklist(&phone, "Shop\nMilk\n✓ Eggs", None)?;
    send(&phone, &desk)?;
    let milk = id_of(&phone, &note, "Milk")?;
    let eggs = id_of(&phone, &note, "Eggs")?;
    Checklists::new(&phone).rename(&milk, "Oat milk")?;
    Checklists::new(&desk).rename(&milk, "Soy milk")?;
    Checklists::new(&desk).uncheck(&eggs)?;
    Checklists::new(&phone).uncheck(&eggs)?;
    Checklists::new(&phone).check(&eggs)?;
    send(&phone, &desk)?;
    send(&desk, &phone)?;
    let conflicts = desk.conflicts()?;
    assert_eq!(conflicts.len(), 1);
    let conflict = &conflicts[0];
    assert_eq!(conflict.entity, milk);
    assert_eq!(conflict.unit, note_keys::TEXT);
    assert_eq!(conflict.entity_type, keys::TYPE_LINE);
    assert!(conflict.resolvable());
    let texts: Vec<String> = conflict.versions.iter().map(|v| v.text().text).collect();
    assert_eq!(texts, ["Soy milk", "Oat milk"]);
    assert!(desk.may_resolve(&milk, &conflict.entity_type)?);
    desk.resolve_conflict(conflict, &conflict.versions[1])?;
    send(&desk, &phone)?;
    assert!(phone.conflicts()?.is_empty());
    assert_eq!(shown(&phone, &note)?, ["Oat milk", "[x] Eggs"]);
    assert_eq!(shown(&desk, &note)?, shown(&phone, &note)?);
    Ok(())
}

#[test]
fn a_checklist_goes_out_in_format_2_signed_or_encrypted_and_arrives_as_one() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let note = checklist(&ann, "Trip\nBath\n  Soap\n✓ Tent", None)?;
    let item = ann.sent_item_of(&note, false)?;
    let to = ben
        .own_key_record()?
        .verified_box_key()
        .ok_or("Ben's X25519 key")?;
    let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
    let key = ann.signing_key()?;
    for file in [
        crate::sent::encode(&item, &key, &[], &mut random)?,
        crate::sent::encode(&item, &key, &[to], &mut random)?,
    ] {
        let outer: serde_json::Value = serde_json::from_slice(&file)?;
        assert_eq!(outer["format"], 2);
        let opened = ben.open_sent_item(&file)?.map_err(|_| "a Sent Item")?;
        let lines: Vec<(String, bool, bool)> = opened
            .item
            .lines
            .iter()
            .flatten()
            .map(|l| (l.text.clone(), l.indent, l.checked))
            .collect();
        assert_eq!(
            lines,
            [
                ("Bath".to_string(), false, false),
                ("Soap".to_string(), true, false),
                ("Tent".to_string(), false, true),
            ]
        );
    }
    let got = ben.accept_sent_item(&item, false)?.ok_or("accepted")?;
    assert!(
        Notes::new(&ben)
            .by_id(&got)?
            .ok_or("the Note")?
            .is_unsorted()
    );
    assert_eq!(shown(&ben, &got)?, ["Bath", "  Soap", "[x] Tent"]);
    let plain = Notes::new(&ann).capture("Plain", None, None)?.id;
    let file = crate::sent::encode(&ann.sent_item_of(&plain, false)?, &key, &[], &mut random)?;
    let outer: serde_json::Value = serde_json::from_slice(&file)?;
    assert!(outer.get("format").is_none());
    Ok(())
}

#[test]
fn lines_come_in_format_2_only_and_a_file_in_format_2_carries_them() -> TestResult {
    use crate::sent::OpenError;
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let note = checklist(&ann, "Trip\nBath", None)?;
    let to = ben
        .own_key_record()?
        .verified_box_key()
        .ok_or("Ben's X25519 key")?;
    let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
    let key = ann.signing_key()?;
    let item = ann.sent_item_of(&note, false)?;
    let plain = Notes::new(&ann).capture("Plain", None, None)?.id;
    let unlisted = ann.sent_item_of(&plain, false)?;
    // The signature covers the item, not the format beside it.
    let refused = |file: Vec<u8>, format: Option<i64>| -> TestResult {
        let mut outer: serde_json::Value = serde_json::from_slice(&file)?;
        let map = outer.as_object_mut().ok_or("an object")?;
        match format {
            Some(format) => map.insert("format".into(), format.into()),
            None => map.remove("format"),
        };
        let file = serde_json::to_vec(&outer)?;
        assert_eq!(
            ben.open_sent_item(&file)?.err(),
            Some(OpenError::NotASentItem)
        );
        Ok(())
    };
    refused(crate::sent::encode(&item, &key, &[], &mut random)?, None)?;
    refused(
        crate::sent::encode(&unlisted, &key, &[], &mut random)?,
        Some(2),
    )?;
    // Encrypted, the outside says it too.
    refused(crate::sent::encode(&item, &key, &[to], &mut random)?, None)?;
    Ok(())
}

#[test]
fn of_two_lines_with_one_id_the_first_counts() -> TestResult {
    use crate::sent::SentLine;
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let note = checklist(&ann, "Trip\nBath\nTent", None)?;
    let mut random = |n: usize| -> Result<Vec<u8>> { Ok(vec![7; n]) };
    let key = ann.signing_key()?;
    let mut item = ann.sent_item_of(&note, false)?;
    let lines = item.lines.clone().ok_or("Lines")?;
    let line = |id: &str, text: &str| SentLine {
        id: id.into(),
        text: text.into(),
        indent: false,
        checked: true,
        reappear: None,
    };
    item.lines = Some(vec![
        lines[0].clone(),
        line(&lines[0].id, "Soap"),
        lines[1].clone(),
    ]);
    let file = crate::sent::encode(&item, &key, &[], &mut random)?;
    let opened = ben.open_sent_item(&file)?.map_err(|_| "a Sent Item")?;
    let texts: Vec<&str> = opened
        .item
        .lines
        .iter()
        .flatten()
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(texts, ["Bath", "Tent"]);
    let got = ben
        .accept_sent_item(&opened.item, false)?
        .ok_or("accepted")?;
    assert_eq!(shown(&ben, &got)?, ["Bath", "Tent"]);
    // An update the same: the Line is there once.
    item.version += 1;
    item.lines = Some(vec![
        lines[0].clone(),
        lines[1].clone(),
        line("line:new", "Map"),
        line("line:new", "Rope"),
    ]);
    let file = crate::sent::encode(&item, &key, &[], &mut random)?;
    let update = ben.open_sent_item(&file)?.map_err(|_| "a Sent Item")?;
    let added: Vec<String> = update
        .changes
        .as_ref()
        .ok_or("changes")?
        .lines_added
        .iter()
        .map(|l| l.text.clone())
        .collect();
    assert_eq!(added, ["Map"]);
    ben.accept_sent_item(&update.item, false)?;
    assert_eq!(shown(&ben, &got)?, ["Bath", "Tent", "[x] Map"]);
    Ok(())
}

#[test]
fn lines_of_text_and_line_breaks_read_as_the_dart_core_reads_them() {
    assert_eq!(
        split_lines("a\r\nb\rc\n\nd\r\r\n"),
        ["a", "b", "c", "", "d", "", ""]
    );
    let row = |text: &str, indent: bool, checked: bool| {
        Some(ParsedLine {
            text: text.into(),
            indent,
            checked,
        })
    };
    assert_eq!(parse_line("   "), None);
    assert_eq!(parse_line(" \u{a0}Nbsp"), row("Nbsp", true, false));
    assert_eq!(parse_line("\u{a0}Lead"), row("Lead", false, false));
    assert_eq!(parse_line("\u{feff}Bom"), row("Bom", false, false));
    assert_eq!(parse_line("  ✓ Pegs"), row("Pegs", true, true));
    assert_eq!(parse_line("✓ "), row("✓", false, false));
    assert_eq!(parse_line("✓x"), row("✓x", false, false));
    assert_eq!(parse_line("- Socks"), row("- Socks", false, false));
    assert_eq!(parse_line_in("- Socks", true), row("Socks", false, false));
    assert_eq!(parse_line_in("  * Blue", true), row("Blue", true, false));
    assert_eq!(parse_line_in("\t+ ✓ Pegs", true), row("Pegs", true, true));
    assert_eq!(parse_line_in("12. Map", true), row("Map", false, false));
    assert_eq!(parse_line_in("> Torch", true), row("Torch", false, false));
    assert_eq!(parse_line_in("- ", true), None);
    assert_eq!(
        parse_line_in("## Later", true),
        row("## Later", false, false)
    );
    assert_eq!(parse_line_in("-Knife", true), row("-Knife", false, false));
    assert_eq!(line_text("a\r\n\nb").ok().as_deref(), Some("a b"));
}

// ------------------------------------------ With a Routine (#121)

/// The wall clock `days` days after the store's now: the tests' clock
/// moves by seconds, the Agenda by days. Day 0 is 2026-01-01.
fn on_day(days: i64) -> impl Fn(DateTime<Utc>) -> chrono::NaiveDateTime {
    move |utc| (utc + Duration::days(days)).naive_utc()
}

fn time(hour: u32) -> Made<crate::schedule::ClockTime> {
    Ok(crate::schedule::ClockTime::new(hour, 0).ok_or("a time")?)
}

fn daily(times: &[crate::schedule::ClockTime]) -> Made<Schedule> {
    let rule = Some(crate::schedule::Recurrence::every(
        crate::schedule::Frequency::Daily,
        1,
    ));
    Ok(Schedule::new(Day::new(2026, 1, 1), times, rule).ok_or("a schedule")?)
}

fn spawn(store: &Store, note: &str, schedule: &Schedule) -> Made<String> {
    Ok(Routines::new(store).spawn(note, schedule)?.id)
}

/// The Occurrence of `routine` on `days` days on, at `time`.
fn on(routine: &str, days: i64, time: Option<crate::schedule::ClockTime>) -> RoutineOccurrence {
    RoutineOccurrence {
        routine_id: routine.to_string(),
        occurrence: Occurrence::new(Day::new(2026, 1, 1).add_days(days), time),
    }
}

/// The Lines as shown for `at`, `[x]` for one checked.
fn shown_for(store: &Store, at: &RoutineOccurrence) -> Made<Vec<String>> {
    let list = Checklists::new(store).for_occurrence(at)?;
    Ok(list
        .map(|l| l.lines)
        .unwrap_or_default()
        .iter()
        .map(|l| {
            format!(
                "{}{}{}",
                if l.in_group() { "  " } else { "" },
                if l.checked { "[x] " } else { "" },
                l.text
            )
        })
        .collect())
}

fn done_at(store: &Store, at: &RoutineOccurrence) -> Made<Option<DateTime<Utc>>> {
    Ok(Routines::new(store)
        .by_id(&at.routine_id)?
        .and_then(|r| r.done_at(&at.occurrence)))
}

#[test]
fn each_occurrence_starts_unchecked_and_is_done_once_every_line_is_checked() -> TestResult {
    let ann = device(1, "Ann")?;
    let note = checklist(&ann, "Gym bag\nTowel\nShoes\n  Laces\nBottle", None)?;
    let lists = Checklists::new(&ann);
    lists.check(&id_of(&ann, &note, "Towel")?)?;
    let routine = spawn(&ann, &note, &daily(&[])?)?;
    let today = on(&routine, 0, None);
    // Its own checks show for no Occurrence.
    assert_eq!(
        shown_for(&ann, &today)?,
        ["Towel", "Shoes", "  Laces", "Bottle"]
    );
    lists.check_for(&id_of(&ann, &note, "Towel")?, &today)?;
    lists.check_for(&id_of(&ann, &note, "Laces")?, &today)?;
    assert_eq!(
        shown_for(&ann, &today)?,
        ["[x] Towel", "[x] Shoes", "  [x] Laces", "Bottle"]
    );
    assert_eq!(done_at(&ann, &today)?, None);
    lists.check_for(&id_of(&ann, &note, "Bottle")?, &today)?;
    let done = done_at(&ann, &today)?;
    assert!(done.is_some());
    let rows = Routines::new(&ann).agenda(&on_day(0))?.today_rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].done(), done);
    let list = lists.for_occurrence(&today)?.ok_or("a Checklist")?;
    assert_eq!(list.on.as_ref(), Some(&today));
    // The next one starts afresh, with nothing written for it.
    let before = ann.all_entries()?.len();
    let tomorrow = on(&routine, 1, None);
    assert_eq!(
        shown_for(&ann, &tomorrow)?,
        ["Towel", "Shoes", "  Laces", "Bottle"]
    );
    assert_eq!(done_at(&ann, &tomorrow)?, None);
    assert_eq!(ann.all_entries()?.len(), before);
    // Every check lies on the Routine; no Line's own check moved.
    assert_eq!(
        lists.by_note(&note)?.ok_or("a Checklist")?.checked_count(),
        1
    );
    // Unchecking a Line undoes the Done; checking it off by hand
    // still works.
    lists.uncheck_for(&id_of(&ann, &note, "Laces")?, &today)?;
    assert_eq!(
        shown_for(&ann, &today)?,
        ["[x] Towel", "Shoes", "  Laces", "[x] Bottle"]
    );
    assert_eq!(done_at(&ann, &today)?, None);
    Routines::new(&ann).mark_done(&routine, &tomorrow.occurrence, None)?;
    assert!(done_at(&ann, &tomorrow)?.is_some());
    // "Uncheck all" for one Occurrence.
    lists.check_for(&id_of(&ann, &note, "Shoes")?, &today)?;
    assert!(done_at(&ann, &today)?.is_some());
    lists.uncheck_all_for(&note, &today)?;
    assert_eq!(
        shown_for(&ann, &today)?,
        ["Towel", "Shoes", "  Laces", "Bottle"]
    );
    assert_eq!(done_at(&ann, &today)?, None);
    Ok(())
}

#[test]
fn a_line_added_after_done_leaves_it_done_and_no_line_is_never_done() -> TestResult {
    let ann = device(1, "Ann")?;
    let note = checklist(&ann, "Shop\nMilk\nEggs", None)?;
    let routine = spawn(&ann, &note, &daily(&[])?)?;
    let lists = Checklists::new(&ann);
    let today = on(&routine, 0, None);
    lists.check_for(&id_of(&ann, &note, "Milk")?, &today)?;
    lists.check_for(&id_of(&ann, &note, "Eggs")?, &today)?;
    let done = done_at(&ann, &today)?;
    assert!(done.is_some());
    let bread = lists.add(&note, "Bread", None, false)?;
    assert_eq!(done_at(&ann, &today)?, done);
    assert_eq!(shown_for(&ann, &today)?, ["[x] Milk", "[x] Eggs", "Bread"]);
    // Checked again since, the Line added counts.
    lists.uncheck_for(&id_of(&ann, &note, "Eggs")?, &today)?;
    lists.check_for(&id_of(&ann, &note, "Eggs")?, &today)?;
    assert_eq!(done_at(&ann, &today)?, None);
    lists.delete(&bread)?;
    assert!(done_at(&ann, &today)?.is_some());
    lists.restore(&bread)?;
    assert_eq!(done_at(&ann, &today)?, None);
    // No Line, no Done, whatever a modified app marks.
    let empty = checklist(&ann, "Empty", None)?;
    let idle = spawn(&ann, &empty, &daily(&[])?)?;
    let field = keys::check_for(&today.occurrence, "line:x");
    ann.append(&idle, &field, Some("1"), None)?;
    assert_eq!(done_at(&ann, &on(&idle, 0, None))?, None);
    let milk = id_of(&ann, &note, "Milk")?;
    assert!(lists.check_for(&milk, &on(&idle, 0, None)).is_err());
    // Back to a plain Note and made again, what was Done stays.
    let tidy = checklist(&ann, "Tidy\nDesk\nBin", None)?;
    let chores = spawn(&ann, &tidy, &daily(&[])?)?;
    let at = on(&chores, 0, None);
    lists.check_for(&id_of(&ann, &tidy, "Desk")?, &at)?;
    lists.check_for(&id_of(&ann, &tidy, "Bin")?, &at)?;
    let done = done_at(&ann, &at)?;
    lists.to_plain(&tidy)?;
    assert_eq!(done_at(&ann, &at)?, done);
    lists.make(&tidy)?;
    assert_eq!(done_at(&ann, &at)?, done);
    assert_eq!(shown_for(&ann, &at)?, ["Desk", "Bin"]);
    Ok(())
}

#[test]
fn checks_no_app_writes_are_passed_over() -> TestResult {
    let ann = device(1, "Ann")?;
    let note = checklist(&ann, "Shop\nMilk", None)?;
    let other = checklist(&ann, "Other\nTea", None)?;
    let routine = spawn(&ann, &note, &daily(&[])?)?;
    let today = on(&routine, 0, None);
    let key = today.occurrence.key();
    let milk = id_of(&ann, &note, "Milk")?;
    let prefix = keys::CHECK_PREFIX;
    // As a modified app would.
    for field in [
        format!("{prefix}{key}"),
        format!("{prefix}@{milk}"),
        format!("{prefix}2026-1-1@{milk}"),
        format!("{prefix}{key}@line:"),
        format!("{prefix}{key}@{milk}@x"),
        format!("{prefix}{key}@note:x"),
        keys::check_for(&today.occurrence, &id_of(&ann, &other, "Tea")?),
    ] {
        ann.append(&routine, &field, Some("2026-01-01T09:00:00.000Z"), None)?;
    }
    assert_eq!(shown_for(&ann, &today)?, ["Milk"]);
    assert_eq!(done_at(&ann, &today)?, None);
    let read = LineChecks::of(&ann, &routine)?;
    assert_eq!(read.by_occurrence.keys().collect::<Vec<_>>(), [&key]);
    // An Occurrence not due, or a Routine of another Note, takes no
    // check.
    let lists = Checklists::new(&ann);
    assert!(
        lists
            .check_for(&milk, &on(&routine, 0, Some(time(8)?)))
            .is_err()
    );
    let elsewhere = spawn(&ann, &other, &daily(&[])?)?;
    assert!(lists.check_for(&milk, &on(&elsewhere, 0, None)).is_err());
    assert_eq!(lists.for_occurrence(&on("routine:none", 0, None))?, None);
    Ok(())
}

#[test]
fn shown_now_is_the_first_not_done_of_the_latest_day_up_to_today() -> TestResult {
    let ann = device(1, "Ann")?;
    let note = checklist(&ann, "Pills\nBlue\nRed", None)?;
    let lists = Checklists::new(&ann);
    let blue = id_of(&ann, &note, "Blue")?;
    let red = id_of(&ann, &note, "Red")?;
    lists.check(&blue)?;
    // Without a Routine, its own checks.
    assert_eq!(lists.occurrence_now(&note, &on_day(0))?, None);
    let now = lists.shown_now(&note, &on_day(0))?.ok_or("a Checklist")?;
    assert_eq!((now.checked_count(), now.on), (1, None));
    let routine = spawn(&ann, &note, &daily(&[time(8)?, time(20)?])?)?;
    let morning = on(&routine, 0, Some(time(8)?));
    assert_eq!(
        lists.occurrence_now(&note, &on_day(0))?,
        Some(morning.clone())
    );
    let now = lists.shown_now(&note, &on_day(0))?.ok_or("a Checklist")?;
    assert_eq!(now.checked_count(), 0);
    lists.check_for(&blue, &morning)?;
    lists.check_for(&red, &morning)?;
    let evening = on(&routine, 0, Some(time(20)?));
    assert_eq!(
        lists.occurrence_now(&note, &on_day(0))?,
        Some(evening.clone())
    );
    Routines::new(&ann).mark_done(&routine, &evening.occurrence, None)?;
    assert_eq!(lists.occurrence_now(&note, &on_day(0))?, Some(evening));
    assert_eq!(
        lists.occurrence_now(&note, &on_day(1))?,
        Some(on(&routine, 1, Some(time(8)?)))
    );
    // A weekly one stays on its day until the next comes; before its
    // first day, the first.
    let weekly = checklist(&ann, "Bins\nPaper", None)?;
    let rule = crate::schedule::Recurrence::parse("FREQ=WEEKLY;BYDAY=MO")?;
    let mondays = spawn(
        &ann,
        &weekly,
        &Schedule::new(Day::new(2026, 1, 5), &[], Some(rule)).ok_or("weekly")?,
    )?;
    for (days, monday) in [(0, 4), (4, 4), (10, 4), (11, 11)] {
        assert_eq!(
            lists.occurrence_now(&weekly, &on_day(days))?,
            Some(on(&mondays, monday, None)),
            "{days}"
        );
    }
    // Of two Routines, the earlier.
    let later = checklist(&ann, "Trip\nTent", None)?;
    let once = |day: i64| Schedule::new(Day::new(2026, 1, 1).add_days(day), &[], None);
    let trip = spawn(&ann, &later, &once(9).ok_or("once")?)?;
    let soon = spawn(&ann, &later, &once(4).ok_or("once")?)?;
    for (days, routine, day) in [(0, &soon, 4), (6, &soon, 4), (9, &trip, 9)] {
        assert_eq!(
            lists.occurrence_now(&later, &on_day(days))?,
            Some(on(routine, day, None)),
            "{days}"
        );
    }
    // In the trash, the Routine leaves it to its own checks.
    Routines::new(&ann).trash(&routine)?;
    assert_eq!(lists.occurrence_now(&note, &on_day(0))?, None);
    let now = lists.shown_now(&note, &on_day(0))?.ok_or("a Checklist")?;
    assert_eq!(now.checked_count(), 1);
    // Many at once, as one by one.
    let plain = Notes::new(&ann).capture("Plain", None, None)?.id;
    let ids = [note, weekly, later, plain];
    let at_once = lists.of_notes_now(&ids, &on_day(1))?;
    assert_eq!(at_once.len(), 3);
    for (id, list) in &at_once {
        assert_eq!(
            Some(list),
            lists.shown_now(id, &on_day(1))?.as_ref(),
            "{id}"
        );
    }
    Ok(())
}

#[test]
fn a_check_another_persons_device_slipped_in_counts_for_nothing() -> TestResult {
    let (ann, ben, _) = household()?;
    let note = checklist(&ann, "Shop\nMilk", None)?;
    let routine = spawn(&ann, &note, &daily(&[])?)?;
    let today = on(&routine, 0, None);
    // As a modified app would: Ben slips a check into Ann's own list.
    let field = keys::check_for(&today.occurrence, &id_of(&ann, &note, "Milk")?);
    let forged = ben.append(&routine, &field, Some("2026-01-01T09:00:00.000Z"), None)?;
    let mut report = ImportReport::default();
    ann.apply_entries(&[forged.wire()], &[], Some(&ben.device_id()?), &mut report)?;
    let held: Vec<String> = ann.all_entries()?.iter().map(Entry::id).collect();
    assert!(held.contains(&forged.id()));
    assert_eq!(shown_for(&ann, &today)?, ["Milk"]);
    assert_eq!(done_at(&ann, &today)?, None);
    Ok(())
}

#[test]
fn in_a_shared_area_a_member_checks_for_everyone() -> TestResult {
    let (ann, ben, area) = household()?;
    let note = checklist(&ann, "Shop\nMilk\nEggs", Some(&area))?;
    let routine = spawn(&ann, &note, &daily(&[])?)?;
    exchange(&ben, &ann)?;
    assert_eq!(ben.area_of(&routine)?, Some(area));
    let today = on(&routine, 0, None);
    Checklists::new(&ben).check_for(&id_of(&ben, &note, "Milk")?, &today)?;
    Checklists::new(&ann).check_for(&id_of(&ann, &note, "Eggs")?, &today)?;
    assert_eq!(done_at(&ann, &today)?, None);
    exchange(&ben, &ann)?;
    exchange(&ben, &ann)?;
    let done = done_at(&ann, &today)?;
    assert!(done.is_some());
    assert_eq!(done_at(&ben, &today)?, done);
    assert_eq!(shown_for(&ann, &today)?, ["[x] Milk", "[x] Eggs"]);
    // The next Occurrence starts afresh for everyone.
    assert_eq!(shown_for(&ben, &on(&routine, 1, None))?, ["Milk", "Eggs"]);
    assert_eq!(
        Checklists::new(&ben).occurrence_now(&note, &on_day(1))?,
        Some(on(&routine, 1, None))
    );
    Ok(())
}

/// Today's Agenda rows of the Checklists of `store`, as "title: open
/// of all".
fn agenda_rows(store: &Store) -> Made<Vec<String>> {
    let notes = Notes::new(store);
    let mut out = Vec::new();
    for row in Checklists::new(store).agenda_rows()? {
        let list = row.list;
        let title = notes.by_id(&list.note_id)?.ok_or("the Note")?.text;
        let suffix = match row.shows {
            ListOnAgenda::Count => " as count",
            _ => "",
        };
        out.push(format!(
            "{title}: {} of {}{suffix}",
            list.open_count(),
            list.count()
        ));
    }
    Ok(out)
}

/// The devices that wrote the "Show on the Agenda" choices `store`
/// holds, and how many there are.
fn choices(store: &Store) -> Made<(Vec<String>, usize)> {
    let mut devices = Vec::new();
    let mut count = 0;
    for e in store.all_entries()? {
        if e.entity.starts_with(keys::AGENDA_PREFIX) {
            count += 1;
            if !devices.contains(&e.device) {
                devices.push(e.device.clone());
            }
        }
    }
    Ok((devices, count))
}

/// `desk` paired with `ann` as one of her own Devices, everything
/// of hers taken in.
pub(crate) fn own_exchange(desk: &Store, ann: &Store) -> TestResult {
    let request = sync_request(desk, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
    let (answer, _) = answer_sync(ann, &request, "Desk")?.ok_or("an answer")?;
    crate::lan::apply_answer(desk, &answer)?;
    for (store, peer) in [(ann, desk), (desk, ann)] {
        store.remember_device(&crate::PairedDevice {
            id: peer.device_id()?,
            name: String::new(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: store.now(),
            shared: false,
        })?;
    }
    Ok(())
}

#[test]
fn a_checklist_shows_on_the_agenda_until_switched_off_or_all_checked() -> TestResult {
    let ann = device(1, "Ann")?;
    let trip = checklist(&ann, "Before the trip\nTent\nTorch\n  Batteries", None)?;
    let lists = Checklists::new(&ann);
    assert_eq!(lists.on_agenda(&trip)?, ListOnAgenda::Lines);
    assert!(lists.may_show_on_agenda(&trip)?);
    assert_eq!(agenda_rows(&ann)?, ["Before the trip: 3 of 3"]);
    lists.check(&id_of(&ann, &trip, "Batteries")?)?;
    assert_eq!(agenda_rows(&ann)?, ["Before the trip: 1 of 3"]);
    lists.set_on_agenda(&trip, ListOnAgenda::Off)?;
    assert_eq!(lists.on_agenda(&trip)?, ListOnAgenda::Off);
    assert!(agenda_rows(&ann)?.is_empty());
    let count = ann.all_entries()?.len();
    lists.set_on_agenda(&trip, ListOnAgenda::Off)?;
    assert_eq!(ann.all_entries()?.len(), count, "the same again");
    lists.set_on_agenda(&trip, ListOnAgenda::Count)?;
    assert_eq!(lists.on_agenda(&trip)?, ListOnAgenda::Count);
    assert_eq!(agenda_rows(&ann)?, ["Before the trip: 1 of 3 as count"]);
    lists.set_on_agenda(&trip, ListOnAgenda::Lines)?;
    assert_eq!(agenda_rows(&ann)?, ["Before the trip: 1 of 3"]);
    let values: Vec<Option<String>> = ann
        .all_entries()?
        .into_iter()
        .filter(|e| e.entity.starts_with(keys::AGENDA_PREFIX))
        .map(|e| e.value)
        .collect();
    assert_eq!(
        values,
        [Some("off"), Some("count"), Some("lines")].map(|v| v.map(String::from))
    );
    lists.check(&id_of(&ann, &trip, "Tent")?)?;
    assert!(agenda_rows(&ann)?.is_empty(), "every Line checked");
    lists.uncheck(&id_of(&ann, &trip, "Tent")?)?;
    assert_eq!(agenda_rows(&ann)?, ["Before the trip: 1 of 3"]);
    let count = ann.all_entries()?.len();
    lists.agenda_rows()?;
    assert_eq!(ann.all_entries()?.len(), count, "reading writes nothing");
    Ok(())
}

#[test]
fn asked_for_a_list_whose_lines_were_all_checked_today_shows_for_the_day() -> TestResult {
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};
    let at = |hour, minute| {
        Utc.with_ymd_and_hms(2026, 3, 5, hour, minute, 0)
            .single()
            .map(|t| t.timestamp())
            .ok_or("a moment")
    };
    let ticks = Arc::new(AtomicI64::new(at(21, 0)?));
    let read = ticks.clone();
    let clock: crate::Clock = Box::new(move || {
        let n = read.fetch_add(1, AtomicOrdering::SeqCst) + 1;
        DateTime::from_timestamp(n, 0).unwrap_or_default()
    });
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([3; 32])))?;
    OwnProfile::new(&store).apply_defaults("Cleo", None)?;
    let wall: &WallClock = &|utc: DateTime<Utc>| (utc + Duration::hours(1)).naive_utc();
    let trip = checklist(&store, "Trip\nTent\nTorch", None)?;
    let lists = Checklists::new(&store);
    lists.check(&id_of(&store, &trip, "Tent")?)?;
    lists.check(&id_of(&store, &trip, "Torch")?)?;
    let today = || -> Made<Vec<String>> {
        let mut out = Vec::new();
        for row in lists.agenda_rows_with(Some(wall))? {
            let day = Day::today(store.now(), wall);
            for line in row.list.lines.iter().filter(|l| l.checked_on(day, wall)) {
                out.push(line.text.clone());
            }
        }
        Ok(out)
    };
    assert!(agenda_rows(&store)?.is_empty(), "unasked");
    assert_eq!(today()?, ["Tent", "Torch"]);
    // Past midnight on the wall clock it shows no more.
    ticks.store(at(23, 30)?, AtomicOrdering::SeqCst);
    assert!(today()?.is_empty());
    assert!(lists.agenda_rows_with(Some(wall))?.is_empty());
    Ok(())
}

#[test]
fn one_shown_as_its_count_never_shows_without_an_open_line() -> TestResult {
    let ann = device(1, "Ann")?;
    let trip = checklist(&ann, "Trip\nTent", None)?;
    let lists = Checklists::new(&ann);
    lists.set_on_agenda(&trip, ListOnAgenda::Count)?;
    assert_eq!(agenda_rows(&ann)?, ["Trip: 1 of 1 as count"]);
    lists.check(&id_of(&ann, &trip, "Tent")?)?;
    assert!(agenda_rows(&ann)?.is_empty());
    Ok(())
}

#[test]
fn a_checklist_nobody_chose_for_shows_every_open_line_writing_nothing() -> TestResult {
    let ann = device(1, "Ann")?;
    let trip = checklist(&ann, "Trip\nTent", None)?;
    let count = ann.all_entries()?.len();
    Checklists::new(&ann).set_on_agenda(&trip, ListOnAgenda::Lines)?;
    assert_eq!(ann.all_entries()?.len(), count);
    assert_eq!(agenda_rows(&ann)?, ["Trip: 1 of 1"]);
    Ok(())
}

#[test]
fn a_checklist_with_a_routine_shows_only_as_its_occurrence() -> TestResult {
    let ann = device(1, "Ann")?;
    let gym = checklist(&ann, "Gym bag\nTowel", None)?;
    let routine = spawn(&ann, &gym, &daily(&[time(7)?])?)?;
    let lists = Checklists::new(&ann);
    assert!(!lists.may_show_on_agenda(&gym)?);
    assert!(agenda_rows(&ann)?.is_empty());
    Routines::new(&ann).trash(&routine)?;
    assert!(lists.may_show_on_agenda(&gym)?);
    assert_eq!(agenda_rows(&ann)?, ["Gym bag: 1 of 1"]);
    Ok(())
}

#[test]
fn a_routine_of_a_later_version_counts_as_none_for_the_agenda() -> TestResult {
    use crate::routines::keys as routine_keys;
    let ann = device(1, "Ann")?;
    let trip = checklist(&ann, "Trip\nTent", None)?;
    ann.batch(|| {
        let id = "routine:later";
        ann.append(
            id,
            routine_keys::TYPE,
            Some(routine_keys::TYPE_ROUTINE),
            None,
        )?;
        ann.append(id, routine_keys::NOTE, Some(&trip), None)?;
        ann.append(id, routine_keys::START, Some("2026-03-01"), None)?;
        ann.append(id, routine_keys::SCHEDULE_VERSION, Some("2"), None)?;
        Ok(())
    })?;
    assert_eq!(Routines::new(&ann).newer_of_note(&trip)?.len(), 1);
    assert!(Checklists::new(&ann).may_show_on_agenda(&trip)?);
    assert_eq!(agenda_rows(&ann)?, ["Trip: 1 of 1"]);
    Ok(())
}

#[test]
fn other_notes_never_show_on_the_agenda_and_the_oldest_comes_first() -> TestResult {
    let ann = device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let lists = Checklists::new(&ann);
    let first = checklist(&ann, "First\nOne", None)?;
    notes.capture("Plain\nline", None, None)?;
    let shop = notes.capture("Groceries\nMilk", None, None)?.id;
    crate::ShoppingLists::new(&ann).make(&shop)?;
    checklist(&ann, "Empty", None)?;
    let trashed = checklist(&ann, "Trashed\nOne", None)?;
    notes.trash(&trashed)?;
    let merged = checklist(&ann, "Merged\nOne", None)?;
    let into = notes.capture("Into", None, None)?.id;
    notes.merge(&merged, &into)?;
    let second = checklist(&ann, "Second\nOne\nTwo", None)?;
    assert_eq!(agenda_rows(&ann)?, ["First: 1 of 1", "Second: 2 of 2"]);
    for id in [&shop, &trashed, &merged, &into] {
        assert!(!lists.may_show_on_agenda(id)?, "{id}");
    }
    assert!(lists.may_show_on_agenda(&first)?);
    assert!(lists.may_show_on_agenda(&second)?);
    assert!(lists.set_on_agenda(&shop, ListOnAgenda::Off).is_err());
    assert!(lists.set_on_agenda(&into, ListOnAgenda::Off).is_err());
    assert_eq!(choices(&ann)?.1, 0);
    Ok(())
}

#[test]
fn an_agenda_choice_no_app_writes_or_none_reads_as_every_open_line() -> TestResult {
    let ann = device(1, "Ann")?;
    let trip = checklist(&ann, "Trip\nTent", None)?;
    let entity = format!("{}{trip}", keys::AGENDA_PREFIX);
    for value in [
        Some("OFF"),
        Some("off "),
        Some("false"),
        Some(""),
        Some("Count"),
        Some("on"),
        None,
    ] {
        ann.append(&entity, keys::AGENDA, value, None)?;
        let shows = Checklists::new(&ann).on_agenda(&trip)?;
        assert_eq!(shows, ListOnAgenda::Lines, "{value:?}");
    }
    assert_eq!(agenda_rows(&ann)?, ["Trip: 1 of 1"]);
    Ok(())
}

#[test]
fn in_a_shared_area_each_person_decides_for_their_own_agenda() -> TestResult {
    let (ann, ben, area) = household()?;
    let trip = checklist(&ann, "Trip\nTent\nTorch", Some(&area))?;
    exchange(&ben, &ann)?;
    assert_eq!(agenda_rows(&ben)?, ["Trip: 2 of 2"]);
    // Ben, a Member, may switch off a list he did not write.
    Checklists::new(&ben).set_on_agenda(&trip, ListOnAgenda::Off)?;
    assert!(agenda_rows(&ben)?.is_empty());
    exchange(&ben, &ann)?;
    assert_eq!(agenda_rows(&ann)?, ["Trip: 2 of 2"]);
    assert_eq!(choices(&ann)?.1, 0);
    // And Ann hers: Ben's shows its count.
    Checklists::new(&ann).set_on_agenda(&trip, ListOnAgenda::Off)?;
    Checklists::new(&ben).set_on_agenda(&trip, ListOnAgenda::Count)?;
    exchange(&ben, &ann)?;
    exchange(&ann, &ben)?;
    assert!(agenda_rows(&ann)?.is_empty());
    assert_eq!(agenda_rows(&ben)?, ["Trip: 2 of 2 as count"]);
    assert_eq!(choices(&ann)?.0, [ann.device_id()?]);
    assert_eq!(choices(&ben)?.0, [ben.device_id()?]);
    Ok(())
}

#[test]
fn ones_own_devices_carry_the_agenda_choice() -> TestResult {
    let ann = device(1, "Ann")?;
    let desk = device(3, "Ann at the desk")?;
    own_exchange(&desk, &ann)?;
    let trip = checklist(&ann, "Trip\nTent", None)?;
    Checklists::new(&ann).set_on_agenda(&trip, ListOnAgenda::Off)?;
    own_exchange(&desk, &ann)?;
    assert!(agenda_rows(&desk)?.is_empty());
    Checklists::new(&desk).set_on_agenda(&trip, ListOnAgenda::Count)?;
    send(&desk, &ann)?;
    assert_eq!(agenda_rows(&ann)?, ["Trip: 1 of 1 as count"]);
    Ok(())
}

#[test]
fn an_agenda_choice_another_persons_device_wrote_counts_for_nothing() -> TestResult {
    let (ann, ben, area) = household()?;
    let trip = checklist(&ann, "Trip\nTent", Some(&area))?;
    exchange(&ben, &ann)?;
    // As a modified app would: Ben takes the list off Ann's Agenda.
    let forged = ben.append(
        &format!("{}{trip}", keys::AGENDA_PREFIX),
        keys::AGENDA,
        Some(keys::AGENDA_OFF),
        None,
    )?;
    ann.apply_entries(
        &[forged.wire()],
        &[],
        Some(&ben.device_id()?),
        &mut ImportReport::default(),
    )?;
    assert_eq!(choices(&ann)?, (vec![ben.device_id()?], 1));
    assert_eq!(Checklists::new(&ann).on_agenda(&trip)?, ListOnAgenda::Lines);
    assert_eq!(agenda_rows(&ann)?, ["Trip: 1 of 1"]);
    // Nor does it stop her switching it off herself.
    Checklists::new(&ann).set_on_agenda(&trip, ListOnAgenda::Off)?;
    assert!(agenda_rows(&ann)?.is_empty());
    Ok(())
}
