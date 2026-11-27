use super::*;
use crate::lan::{
    Joining, answer_sync, answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
};
use crate::signing::SigningKey;
use crate::store::tests::ticking;
use crate::{Notes, Places, Routines};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// A Device with its own Profile: Ann's, Ben's or the laptop's.
fn device(seed: u8) -> Result<Store> {
    let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
    let name = ["", "Ann", "Ben", "Laptop"][usize::from(seed).min(3)];
    crate::profile::OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

#[test]
fn a_member_writes_only_what_a_member_may() -> TestResult {
    let ann = device(1)?;
    let ben = device(2)?;
    let house = Places::new(&ann).add_area("Household")?;
    let milk = Notes::new(&ann).capture("Buy milk", Some(&house.id), None)?;
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
    assert!(!ben.may_change(&milk.id)?);
    // Written all the same, as a modified app would: ignored.
    Notes::new(&ben).edit(&milk.id, "Buy beer")?;
    let own = Notes::new(&ben).capture("Bread", Some(&area), None)?;
    assert!(ben.may_change(&own.id)?);
    assert!(Places::new(&ben).add_topic(&area, "Garage").is_err());
    assert!(
        Routines::new(&ben)
            .spawn(
                &own.id,
                &crate::Schedule {
                    start: crate::Day::new(2026, 3, 1),
                    times: Vec::new(),
                    rule: None,
                }
            )
            .is_err()
    );
    assert_eq!(
        Notes::new(&ben).by_id(&milk.id)?.map(|n| n.text),
        Some("Buy milk".into())
    );
    assert!(!ben.refuses_invite("area:nothing")?);
    Ok(())
}

#[test]
fn an_own_device_is_not_invited() -> TestResult {
    let ann = device(1)?;
    let laptop = device(3)?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    ann.remember_device(&crate::PairedDevice {
        id: laptop.device_id()?,
        name: "Laptop".into(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: ann.now(),
        shared: false,
    })?;
    let invite = Invite {
        area,
        role: AreaRole::Member,
    };
    let invited = sync_request_as(
        &laptop,
        &ann.version_vector()?,
        "Laptop",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    assert!(answer_sync_inviting(&ann, &invited, "Desk", Some(&invite))?.is_none());
    let host_profile = crate::lan::profile_answer(&ann)?;
    let inviting = sync_request_as(
        &laptop,
        &ann.version_vector()?,
        "Laptop",
        &ann.device_id()?,
        "",
        Joining::Inviting(&invite, host_profile.get("profile")),
    )?;
    assert!(answer_sync(&ann, &inviting, "Desk")?.is_none());
    assert!(!ann.is_shared_peer(&laptop.device_id()?)?);
    // Nor is it granted anything as another person.
    ann.invite(&invite.area, &laptop.device_id()?, AreaRole::Member, "ben")?;
    let mut names = Vec::new();
    for p in ann.people(&invite.area)? {
        names.push(ann.person_shown(&p.id)?.name);
    }
    assert_eq!(names, vec!["Ann".to_string()]);
    Ok(())
}

#[test]
fn an_own_device_met_in_a_sync_gets_the_same_role() -> TestResult {
    let ann = device(1)?;
    let laptop = device(3)?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    let request = sync_request(&laptop, &ann.version_vector()?, "Laptop", "", "")?;
    let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    crate::lan::apply_answer(&laptop, &answer)?;
    let id = laptop.device_id()?;
    assert_eq!(laptop.access_in(&area, None)?, AreaAccess::Manager);
    assert_eq!(ann.access_in(&area, Some(&id))?, AreaAccess::Manager);
    assert_eq!(ann.people(&area)?.len(), 1);
    assert_eq!(ann.extend_roles_to(&id)?, 0, "given once");
    Ok(())
}

#[test]
fn a_grant_reads_as_newer_by_its_version_its_keys_or_its_role() {
    for (value, role) in [
        (r#"{"profile":"p","role":"member"}"#, AreaRole::Member),
        (
            r#"{"profile":"p","role":"manager","v":1}"#,
            AreaRole::Manager,
        ),
    ] {
        assert_eq!(
            Grant::parse(value).and_then(|g| g.role),
            Some(role),
            "{value}"
        );
    }
    for value in [
        r#"{"profile":"p","role":"viewer"}"#,
        r#"{"profile":"p","role":"member","v":2}"#,
        r#"{"profile":"p","role":"member","v":1.5}"#,
        r#"{"profile":"p","role":"member","v":1.0}"#,
        r#"{"profile":"p","role":"member","v":"1"}"#,
        r#"{"profile":"p","role":"member","v":0}"#,
        r#"{"profile":"p","role":"member","v":null}"#,
        r#"{"profile":"p","role":"member","until":"2027"}"#,
        r#"{"profile":"p","role":3}"#,
        r#"{"profile":"p"}"#,
    ] {
        let grant = Grant::parse(value);
        assert_eq!(grant.as_ref().map(Grant::newer), Some(true), "{value}");
        assert_eq!(grant.map(|g| g.profile).as_deref(), Some("p"), "{value}");
    }
    for value in [
        r#"{"profile":"","role":"member"}"#,
        r#"{"profile":3,"role":"member"}"#,
        r#"{"role":"member"}"#,
        r#"["p","member"]"#,
        "member",
        "",
    ] {
        assert_eq!(Grant::parse(value), None, "{value}");
    }
    let newer = Grant {
        role: None,
        profile: "p".into(),
    };
    assert_eq!(newer.encode(), None);
}

#[test]
fn a_person_whose_grant_is_newer_keeps_the_area_and_nothing_here_changes_it() -> TestResult {
    let ann = device(1)?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    // As Ann's Device of a later version writes it.
    let viewer = r#"{"profile":"ben","role":"viewer","v":2}"#;
    ann.append(&area, &role_field("ben"), Some(viewer), None)?;
    let people = ann.people(&area)?;
    let ben = people.iter().find(|p| p.id == "ben").ok_or("Ben")?;
    assert_eq!(ben.role, None);
    assert_eq!(ann.access_in(&area, Some("ben"))?, AreaAccess::None);
    assert!(ann.areas_shared_with("ben")?.contains(&area));
    let before = ann.history(&area)?.len();
    assert!(ann.set_role(&area, "ben", AreaRole::Manager).is_err());
    assert!(ann.remove_person(&area, "ben").is_err());
    ann.invite(&area, "ben", AreaRole::Member, "ben")?;
    assert_eq!(ann.history(&area)?.len(), before);
    Ok(())
}

#[test]
fn a_device_whose_grant_is_newer_grants_nothing_and_writes_nothing_that_counts() -> TestResult {
    let ann = device(1)?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    let me = ann.device_id()?;
    let profile = ann
        .people(&area)?
        .first()
        .map(|p| p.id.clone())
        .ok_or("Ann")?;
    let viewer = format!(r#"{{"profile":"{profile}","role":"viewer"}}"#);
    ann.append(&area, &role_field(&me), Some(&viewer), None)?;
    // As a Member adds an own Device, by hand as a modified app would.
    let member = encode_grant(AreaRole::Member, &profile);
    ann.append(&area, &role_field("laptop"), Some(&member), None)?;
    ann.append(&area, place_keys::NAME, Some("Not counted"), None)?;
    let grants = ann.grants_of(&area)?.ok_or("shared")?;
    assert_eq!(grants.grant_of("laptop"), None);
    assert_eq!(ann.access_in(&area, Some("laptop"))?, AreaAccess::None);
    assert_eq!(
        Places::new(&ann).area_by_id(&area)?.map(|a| a.name),
        Some("Household".into())
    );
    assert!(!ann.may_write_in(&area)?);
    assert_eq!(ann.extend_roles_to("peer")?, 0);
    Ok(())
}

/// Fails unless what `store` holds as judged is what the whole log
/// judges now (obf-1092).
fn assert_judged_as_all(store: &Store) -> Result<()> {
    let held = store.judged()?;
    let entries = store.all_entries()?;
    let all = judge_log(&entries);
    assert_eq!(held.ignored, all.ignored);
    assert_eq!(
        held.areas.keys().collect::<Vec<_>>(),
        all.areas.keys().collect::<Vec<_>>()
    );
    for (id, grants) in &all.areas {
        let kept = held
            .areas
            .get(id)
            .ok_or_else(|| Error::Invalid(format!("{id} is not judged")))?;
        assert_eq!(kept.first.id(), grants.first.id());
        assert_eq!(kept.devices(), grants.devices());
        for device in grants.devices() {
            assert_eq!(
                kept.grant_of(&device).map(|g| g.role),
                grants.grant_of(&device).map(|g| g.role)
            );
            assert_eq!(kept.person_of(&device), grants.person_of(&device));
            for e in &entries {
                assert_eq!(kept.access_at(&device, e), grants.access_at(&device, e));
            }
        }
    }
    Ok(())
}

fn no_times() -> crate::Schedule {
    crate::Schedule {
        start: crate::Day::new(2026, 3, 1),
        times: Vec::new(),
        rule: None,
    }
}

#[test]
fn the_first_grant_written_inside_a_batch_counts_at_once() -> TestResult {
    let ann = device(1)?;
    let house = Places::new(&ann).add_area("Household")?;
    ann.batch(|| {
        // What was judged before the batch goes with its first entry.
        Places::new(&ann).rename(&house.id, "Home")?;
        assert!(!ann.is_shared(&house.id)?);
        ann.share(&house.id)?;
        assert!(ann.is_shared(&house.id)?);
        assert_eq!(ann.access_in(&house.id, None)?, AreaAccess::Manager);
        assert_judged_as_all(&ann)
    })?;
    Ok(())
}

#[test]
fn a_batch_of_writes_with_reads_between_them_judges_the_log_once() -> TestResult {
    use std::sync::atomic::Ordering::SeqCst;
    let ann = device(1)?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    let old = Places::new(&ann).add_area("Old")?;
    for i in 0..4 {
        let note = Notes::new(&ann).capture(&format!("Old {i}"), Some(&old.id), None)?;
        Routines::new(&ann).spawn(&note.id, &no_times())?;
    }
    let before = ann.judged_builds.load(SeqCst);
    Places::new(&ann).delete_area(&old.id, crate::NotesGo::Trash)?;
    assert!(ann.judged_builds.load(SeqCst) - before <= 1);
    // As an import writes: new Notes, each read back.
    let before = ann.judged_builds.load(SeqCst);
    ann.batch(|| {
        for i in 0..4 {
            let id = format!("note:imported{i}");
            ann.append(&id, note_keys::TYPE, Some(note_keys::TYPE_NOTE), None)?;
            ann.append(&id, note_keys::PLACE, Some(&area), None)?;
            ann.append(&id, note_keys::TEXT, Some(&format!("Imported {i}")), None)?;
            assert_eq!(ann.current(&id, note_keys::PLACE)?, Some(area.clone()));
            assert_judged_as_all(&ann)?;
        }
        Ok(())
    })?;
    assert_eq!(ann.judged_builds.load(SeqCst) - before, 1);
    Ok(())
}

/// Ann's household with a Topic, a Note in it and its Routine, shared
/// with Ben as a Member, and her private Health beside it.
struct Household {
    area: String,
    kitchen: String,
    milk: String,
    bins: String,
    health: String,
}

fn household(
    ann: &Store,
    ben: &Store,
) -> std::result::Result<Household, Box<dyn std::error::Error>> {
    let places = Places::new(ann);
    let house = places.add_area("Household")?;
    let kitchen = places.add_topic(&house.id, "Kitchen")?;
    let milk = Notes::new(ann).capture("Buy milk", Some(&kitchen.id), None)?;
    let bins = Routines::new(ann).spawn(&milk.id, &no_times())?;
    let health = places.add_area("Health")?;
    let area = ann.share(&house.id)?;
    crate::checklists::tests::invite(ann, ben, &area)?;
    Ok(Household {
        area,
        kitchen: kitchen.id,
        milk: milk.id,
        bins: bins.id,
        health: health.id,
    })
}

#[test]
fn what_a_managers_batch_judges_again_is_what_the_whole_log_judges() -> TestResult {
    use chrono::TimeZone;
    use std::sync::atomic::Ordering::SeqCst;
    let ann = device(1)?;
    let ben = device(2)?;
    let h = household(&ann, &ben)?;
    let may = chrono::Utc
        .with_ymd_and_hms(2026, 5, 1, 0, 0, 0)
        .single()
        .ok_or("a date")?;
    let june = may + chrono::Duration::days(31);
    // Not Ben's to do, written all the same after everything else:
    // ignored.
    ben.append(&h.bins, routine_keys::START, Some("2026-03-05"), Some(june))?;
    let beer = ben.append(&h.milk, note_keys::TEXT, Some("Buy beer"), Some(june))?;
    let own = Notes::new(&ben).capture("Ben buys bread", Some(&h.kitchen), None)?;
    Notes::new(&ben).amend(&own.id, "Rye")?;
    crate::checklists::tests::exchange(&ben, &ann)?;
    assert!(ann.judged()?.ignored.contains(&beer.id()));
    let before = ann.judged_builds.load(SeqCst);
    ann.batch(|| {
        assert_judged_as_all(&ann)?;
        // Filed in Health before Ben wrote: what he wrote on the Note and
        // its Routine lies in no Shared Area then, and counts.
        ann.append(&h.milk, note_keys::PLACE, Some(&h.health), Some(may))?;
        assert_judged_as_all(&ann)?;
        assert!(!ann.judged()?.ignored.contains(&beer.id()));
        ann.append(
            &h.milk,
            note_keys::PLACE,
            Some(&h.kitchen),
            Some(may + chrono::Duration::milliseconds(1)),
        )?;
        assert_judged_as_all(&ann)?;
        assert!(ann.judged()?.ignored.contains(&beer.id()));
        // A first `$type` dated before Ben's.
        let early = chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| Error::Invalid("a date".into()))?;
        ann.append(
            &own.id,
            note_keys::TYPE,
            Some(note_keys::TYPE_NOTE),
            Some(early),
        )?;
        assert_judged_as_all(&ann)?;
        ann.append(&h.kitchen, place_keys::AREA, Some(&h.health), Some(may))?;
        assert_judged_as_all(&ann)?;
        ann.share(&h.health)?;
        assert_judged_as_all(&ann)?;
        ann.append(&h.area, &role_field(&ben.device_id()?), None, None)?;
        assert_judged_as_all(&ann)?;
        Places::new(&ann).rename(&h.area, "Home")?;
        Notes::new(&ann).trash(&h.milk)?;
        assert_judged_as_all(&ann)?;
        Ok(())
    })?;
    assert_eq!(ann.judged_builds.load(SeqCst) - before, 1);
    assert_judged_as_all(&ann)?;
    Ok(())
}

#[test]
fn what_a_members_batch_judges_again_is_what_the_whole_log_judges() -> TestResult {
    use std::sync::atomic::Ordering::SeqCst;
    let ann = device(1)?;
    let ben = device(2)?;
    let h = household(&ann, &ben)?;
    let before = ben.judged_builds.load(SeqCst);
    ben.batch(|| {
        assert_judged_as_all(&ben)?;
        Notes::new(&ben).edit(&h.milk, "Buy beer")?;
        // Ignored at once, inside the batch too.
        assert_eq!(
            Notes::new(&ben).by_id(&h.milk)?.map(|n| n.text),
            Some("Buy milk".into())
        );
        assert_judged_as_all(&ben)?;
        let own = Notes::new(&ben).capture("Ben buys bread", Some(&h.kitchen), None)?;
        Notes::new(&ben).amend(&h.milk, "Oat milk please")?;
        assert_judged_as_all(&ben)?;
        // As a modified app would write them.
        ben.append(
            "routine:ben",
            routine_keys::TYPE,
            Some(routine_keys::TYPE_ROUTINE),
            None,
        )?;
        ben.append("routine:ben", routine_keys::NOTE, Some(&own.id), None)?;
        ben.append("topic:garage", place_keys::AREA, Some(&h.area), None)?;
        assert_judged_as_all(&ben)?;
        ben.append(
            &h.area,
            &role_field(&ben.device_id()?),
            Some(&encode_grant(AreaRole::Manager, "ben")),
            None,
        )?;
        assert_judged_as_all(&ben)?;
        ben.append(&own.id, note_keys::PLACE, None, None)?;
        assert_judged_as_all(&ben)?;
        Ok(())
    })?;
    assert_eq!(ben.judged_builds.load(SeqCst) - before, 1);
    Ok(())
}

#[test]
fn a_topic_keeps_its_icon_when_its_area_moves_to_fresh_ids() -> TestResult {
    let ann = device(1)?;
    let places = Places::new(&ann);
    places.plant_starter_set(&["household", "kitchen"])?;
    let starter = crate::places::starter_id(place_keys::TYPE_AREA, "household");
    let kitchen = crate::places::starter_id(place_keys::TYPE_TOPIC, "kitchen");
    places.set_topic_icon(&kitchen, Some("food"))?;
    let area = ann.share(&starter)?;
    assert_ne!(area, starter);
    let icons: Vec<_> = places.topics(&area)?.into_iter().map(|t| t.icon).collect();
    assert_eq!(icons, [Some("food".to_string())]);
    Ok(())
}
