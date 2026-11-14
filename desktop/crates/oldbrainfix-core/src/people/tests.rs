use super::*;
use crate::lan::{
    Joining, answer_sync, answer_sync_inviting, apply_answer_as, profile_answer, sync_request,
    sync_request_as,
};
use crate::profile::{ProfileKey, key_bytes, record_bytes};
use crate::sent::SentItemState;
use crate::shared::{AreaRole, Invite};
use crate::signing::SigningKey;
use crate::store::tests::ticking;
use crate::{Notes, PairedDevice, Places};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// A Device with its own Profile, named by the user.
fn device(seed: u8, name: &str) -> Result<Store> {
    let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
    let own = OwnProfile::new(&store);
    own.apply_defaults("Device", None)?;
    own.rename(name)?;
    Ok(store)
}

fn remember(store: &Store, peer: &Store, shared: bool) -> TestResult {
    store.remember_device(&PairedDevice {
        id: peer.device_id()?,
        name: String::new(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: store.now(),
        shared,
    })?;
    Ok(())
}

/// Ann's Household shared with Ben as a Member: Ann's desk invites
/// Ben's phone, then they sync once more as paired.
fn household(ann: &Store, ben: &Store) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let house = Places::new(ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    let invite = Invite {
        area: area.clone(),
        role: AreaRole::Member,
    };
    let request = sync_request_as(
        ben,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let (answer, session) =
        answer_sync_inviting(ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    assert!(session.people_changed);
    apply_answer_as(ben, &answer, true, None)?;
    remember(ann, ben, true)?;
    remember(ben, ann, true)?;
    Ok(area)
}

fn own_id(store: &Store) -> std::result::Result<String, Box<dyn std::error::Error>> {
    Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
}

#[test]
fn a_sync_through_another_connection_shows_once_it_says_so() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("oldbrainfix.db");
    let ann = Store::open(&path, ticking(), || Ok(SigningKey::from_seed([1; 32])))?;
    OwnProfile::new(&ann).apply_defaults("Device", None)?;
    OwnProfile::new(&ann).rename("Ann")?;
    // The sync's own connection, as its thread opens it.
    let sync = Store::open(&path, ticking(), || {
        Err(crate::Error::Invalid("the same Device".into()))
    })?;
    let ben = device(2, "Ben")?;
    assert!(ann.kept_people()?.is_empty());
    household(&sync, &ben)?;
    assert!(
        ann.kept_people()?.is_empty(),
        "what was read stays until told"
    );
    ann.changed_elsewhere();
    let names: Vec<String> = ann.kept_people()?.into_iter().map(|p| p.name).collect();
    assert_eq!(names, ["Ben"]);
    Ok(())
}

#[test]
fn an_invite_carries_both_profiles() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let area = household(&ann, &ben)?;
    assert_eq!(ann.person_shown(&own_id(&ben)?)?.name, "Ben");
    assert_eq!(ben.person_shown(&own_id(&ann)?)?.name, "Ann");
    assert_eq!(
        ben.person_code(&own_id(&ann)?)?,
        OwnProfile::new(&ann).code()?
    );
    let mut names = Vec::new();
    for p in ann.people(&area)? {
        names.push(ann.person_shown(&p.id)?.name);
    }
    assert_eq!(names, ["Ann", "Ben"]);
    // A renamed Profile reaches the other with the next sync.
    OwnProfile::new(&ben).rename("Benjamin")?;
    let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
    let (_, session) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    assert!(session.people_changed);
    assert_eq!(ann.person_shown(&own_id(&ben)?)?.name, "Benjamin");
    Ok(())
}

#[test]
fn a_profiles_file_of_a_later_version_is_not_read() -> TestResult {
    use crate::folder::{MemoryFolder, SyncFolder, folder_sync, profiles_file_name, random_nonce};
    use crate::keys::{seal, sealed_as, unseal};
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let area = household(&ann, &ben)?;
    let mut folder = MemoryFolder::default();
    OwnProfile::new(&ann).rename("Ann A.")?;
    folder_sync(&ann, &mut folder, &mut random_nonce)?;
    let key = ann
        .area_keys()?
        .into_iter()
        .find(|k| k.scope == area)
        .ok_or("the Area's key")?;
    let own = ann.device_id()?;
    let name = profiles_file_name(&own, &key.id());
    let aad = sealed_as::profiles(&own);
    let held = folder.read("keys", &name)?.ok_or("a Profiles file")?;
    let plain = unseal(&key.key, &held, &aad).ok_or("opens")?;
    let mut json: Value = serde_json::from_slice(&plain)?;
    if let Some(map) = json.as_object_mut() {
        map.insert("v".into(), serde_json::json!(2));
    }
    let later = seal(
        &key.key,
        json.to_string().as_bytes(),
        &aad,
        &random_nonce()?,
    )?;
    folder.write("keys", &name, &later)?;
    let result = folder_sync(&ben, &mut folder, &mut random_nonce)?;
    assert_eq!(result.newer, 1);
    assert!(!result.profile_changed);
    assert_eq!(ben.person_shown(&own_id(&ann)?)?.name, "Ann");
    // Anyone holding the Area's key could have written that one: the
    // own is written over as before.
    folder_sync(&ann, &mut folder, &mut random_nonce)?;
    let again = folder.read("keys", &name)?.ok_or("a Profiles file")?;
    assert_eq!(unseal(&key.key, &again, &aad), Some(plain));
    Ok(())
}

#[test]
fn an_update_signed_by_a_key_not_bound_is_refused() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    household(&ann, &ben)?;
    let sent = OwnProfile::new(&ben).signed()?.ok_or("a Profile")?;
    let id = own_id(&ben)?;
    let mallory = SigningKey::from_seed([9; 32]);
    let mallory_key = BASE64.encode(mallory.public_key());
    let record = Profile {
        id: id.clone(),
        name: "Mallory".into(),
        picture: None,
        version: 9,
    }
    .encode();
    let mut keys = sent.keys.clone();
    keys.push(ProfileKey {
        key: mallory_key.clone(),
        by: mallory_key.clone(),
        sig: mallory.sign(&key_bytes(&id, &mallory_key)),
    });
    let forged = SignedProfile {
        sig: mallory.sign(&record_bytes(&record)),
        record,
        signer: mallory_key,
        first: sent.first.clone(),
        keys,
        aliases: Vec::new(),
        fresh: false,
    };
    assert_eq!(
        ann.take_person(&forged.to_json(), None)?,
        PersonTaken::Refused
    );
    assert_eq!(ann.person_shown(&id)?.name, "Ben");
    OwnProfile::new(&ben).rename("Ben B.")?;
    let update = ben.own_profile_for_others()?.ok_or("a Profile")?;
    assert_eq!(ann.take_person(&update, None)?, PersonTaken::Updated);
    assert_eq!(ann.person_shown(&id)?.name, "Ben B.");
    Ok(())
}

#[test]
fn a_received_name_is_sanitised_again() -> TestResult {
    let ann = device(1, "Ann")?;
    let key = SigningKey::from_seed([5; 32]);
    let record = r#"{"id":"cat","name":" \u202eCat\u200b\u0007 Lee ","version":1}"#;
    let sent = SignedProfile {
        record: record.into(),
        signer: BASE64.encode(key.public_key()),
        sig: key.sign(&record_bytes(record)),
        first: BASE64.encode(key.public_key()),
        keys: Vec::new(),
        aliases: Vec::new(),
        fresh: false,
    };
    assert_eq!(ann.take_person(&sent.to_json(), None)?, PersonTaken::Added);
    assert_eq!(ann.person_shown("cat")?.name, "Cat Lee");
    Ok(())
}

#[test]
fn notes_are_marked_by_the_profile_the_grant_names() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let area = household(&ann, &ben)?;
    let mine = Notes::new(&ann).capture("Milk", Some(&area), None)?;
    let theirs = Notes::new(&ben).capture("Eggs", Some(&area), None)?;
    let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
    let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    assert_eq!(ann.author_of(&mine.id)?, None);
    assert_eq!(
        ann.author_of(&theirs.id)?.map(|p| p.name),
        Some("Ben".into())
    );
    assert_eq!(ben.author_of(&theirs.id)?, None);
    assert_eq!(ben.author_of(&mine.id)?.map(|p| p.name), Some("Ann".into()));
    // A Profile that binds Ben's key into its own marks none of his
    // Notes.
    let mallory = SigningKey::from_seed([9; 32]);
    let mallory_key = BASE64.encode(mallory.public_key());
    let ben_key = BASE64.encode(ben.signing_key()?.public_key());
    let id = "0".repeat(32);
    let record = Profile {
        id: id.clone(),
        name: "Mallory".into(),
        picture: None,
        version: 1,
    }
    .encode();
    let claim = SignedProfile {
        sig: mallory.sign(&record_bytes(&record)),
        record,
        signer: mallory_key.clone(),
        first: mallory_key.clone(),
        keys: vec![ProfileKey {
            key: ben_key.clone(),
            by: mallory_key,
            sig: mallory.sign(&key_bytes(&id, &ben_key)),
        }],
        aliases: Vec::new(),
        fresh: false,
    };
    assert_eq!(ann.take_person(&claim.to_json(), None)?, PersonTaken::Added);
    assert_eq!(
        ann.author_of(&theirs.id)?.map(|p| p.name),
        Some("Ben".into())
    );
    Ok(())
}

#[test]
fn a_file_is_named_by_the_person_whose_device_added_it() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let area = household(&ann, &ben)?;
    let file = |name| crate::NewFile {
        name,
        media_type: "",
        bytes: name.as_bytes(),
        pages: None,
    };
    let mine = Notes::new(&ann).capture_with_files(
        "Milk",
        Some(&area),
        None,
        &[],
        &[],
        &[file("list.txt")],
    )?;
    let theirs = Notes::new(&ben).capture_with_files(
        "Eggs",
        Some(&area),
        None,
        &[],
        &[],
        &[file("setup.exe")],
    )?;
    sync(&ben, &ann)?;
    let hash = |note: &Note| note.attachments.first().map(|a| a.hash.clone());
    let mine_hash = hash(&mine).ok_or("a file")?;
    let theirs_hash = hash(&theirs).ok_or("a file")?;
    assert_eq!(ann.file_added_by(&mine.id, &mine_hash)?, None);
    assert_eq!(
        ann.file_added_by(&theirs.id, &theirs_hash)?.map(|p| p.name),
        Some("Ben".into())
    );
    assert_eq!(ben.file_added_by(&theirs.id, &theirs_hash)?, None);
    assert_eq!(
        ben.file_added_by(&mine.id, &mine_hash)?.map(|p| p.name),
        Some("Ann".into())
    );
    // A file the Note does not hold was added by nobody.
    assert_eq!(ann.file_added_by(&theirs.id, &mine_hash)?, None);
    Ok(())
}

fn ids(notes: &[Note]) -> Vec<String> {
    notes.iter().map(|n| n.id.clone()).collect()
}

fn me() -> PersonFilter {
    PersonFilter {
        me: true,
        people: BTreeSet::new(),
    }
}

#[test]
fn the_person_filter_keeps_me_them_or_both_with_the_search() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let area = household(&ann, &ben)?;
    let notes = Notes::new(&ann);
    let mine = notes.capture("Milk and eggs", Some(&area), None)?;
    let unsorted = notes.capture("Eggs for the cake", None, None)?;
    let theirs = Notes::new(&ben).capture("Eggs", Some(&area), None)?;
    let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
    let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    let all = notes.all()?;
    let ben_only = PersonFilter::person(&own_id(&ben)?);
    assert_eq!(
        ids(&ann.notes_by(all.clone(), &PersonFilter::default())?),
        ids(&all)
    );
    assert_eq!(
        ids(&ann.notes_by(all.clone(), &me())?),
        vec![unsorted.id.clone(), mine.id.clone()]
    );
    assert_eq!(
        ids(&ann.notes_by(all.clone(), &ben_only)?),
        vec![theirs.id.clone()]
    );
    let mut both = me();
    both.toggle(&own_id(&ben)?);
    assert_eq!(ids(&ann.notes_by(all.clone(), &both)?), ids(&all));
    // The own Profile's id among the people counts as "Me".
    assert_eq!(
        ids(&ann.notes_by(all.clone(), &PersonFilter::person(&own_id(&ann)?))?),
        vec![unsorted.id.clone(), mine.id.clone()]
    );
    // Ben sees it the other way round.
    let on_ben = Notes::new(&ben).all()?;
    assert_eq!(
        ids(&ben.notes_by(on_ben.clone(), &me())?),
        vec![theirs.id.clone()]
    );
    assert_eq!(
        ids(&ben.notes_by(on_ben, &PersonFilter::person(&own_id(&ann)?))?),
        vec![mine.id.clone()]
    );
    // With the search over the Area.
    let found = crate::Search::new(&ann).notes("eggs", Some(&area))?;
    assert_eq!(found.len(), 2);
    assert_eq!(ids(&ann.notes_by(found.clone(), &me())?), vec![mine.id]);
    assert_eq!(ids(&ann.notes_by(found, &ben_only)?), vec![theirs.id]);
    Ok(())
}

#[test]
fn a_note_whose_writer_is_not_known_is_nobodys() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let area = household(&ann, &ben)?;
    let theirs = Notes::new(&ben).capture("Eggs", Some(&area), None)?;
    let moved = Notes::new(&ben).capture("Flour", Some(&area), None)?;
    let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
    answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    // Ann takes one of Ben's Notes into an Area of her own.
    let baking = Places::new(&ann).add_area("Baking")?;
    Notes::new(&ann).file(&moved.id, &baking.id)?;
    let ben_id = own_id(&ben)?;
    assert_eq!(
        ann.author_of(&moved.id)?.map(|p| p.id),
        Some(ben_id.clone())
    );
    // Another Device of Ann's, met without the Profiles.
    let pad = device(3, "Ann on the pad")?;
    let request = sync_request(&pad, &ann.version_vector()?, "Pad", &ann.device_id()?, "")?;
    let (mut answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    answer
        .as_object_mut()
        .ok_or("an object")?
        .remove("profiles");
    crate::lan::apply_answer(&pad, &answer)?;
    assert!(pad.kept_people()?.is_empty());
    let ben_only = PersonFilter::person(&ben_id);
    // In the Area its grant names Ben, before his Profile arrives.
    assert!(!pad.wrote_by(&theirs.id, &me())?);
    assert!(pad.wrote_by(&theirs.id, &ben_only)?);
    // Outside it no grant names him and no Profile binds his key here.
    assert_eq!(pad.author_of(&moved.id)?, None);
    assert!(!pad.wrote_by(&moved.id, &me())?);
    assert!(!pad.wrote_by(&moved.id, &ben_only)?);
    // Nothing in the log says who wrote it.
    let nothing = "0".repeat(32);
    assert!(!pad.wrote_by(&nothing, &me())?);
    assert!(!pad.wrote_by(&nothing, &ben_only)?);
    Ok(())
}

/// Ann's Household shared with Ben's desk, invited on its own under
/// its own Profile, which wrote a Note there.
fn desk_with_a_note(
    ann: &Store,
    desk: &Store,
) -> std::result::Result<(String, String), Box<dyn std::error::Error>> {
    let area = household(ann, desk)?;
    let note = Notes::new(desk).capture("Fix the tap", Some(&area), None)?;
    let request = sync_request(desk, &ann.version_vector()?, "Desk", &ann.device_id()?, "")?;
    answer_sync(ann, &request, "Desk")?.ok_or("an answer")?;
    Ok((area, note.id))
}

#[test]
fn a_note_is_marked_through_the_aliases_of_its_writers_profile() -> TestResult {
    let ann = device(1, "Ann")?;
    let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([4; 32])))?;
    OwnProfile::new(&desk).apply_defaults("Ben at the desk", None)?;
    let desk_id = own_id(&desk)?;
    let (_, note) = desk_with_a_note(&ann, &desk)?;
    assert_eq!(
        ann.author_of(&note)?.map(|p| p.name),
        Some("Ben at the desk".into())
    );
    // Ben pairs it with his phone, whose Profile it takes over.
    let ben = device(2, "Ben")?;
    for _ in 0..2 {
        let request = sync_request(&ben, &desk.version_vector()?, "Phone", "", "")?;
        let (answer, _) = answer_sync(&desk, &request, "Desk")?.ok_or("an answer")?;
        crate::lan::apply_answer(&ben, &answer)?;
    }
    assert_eq!(own_id(&desk)?, own_id(&ben)?);
    let request = sync_request(&desk, &ann.version_vector()?, "Desk", &ann.device_id()?, "")?;
    answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    assert_eq!(ann.resolve_person(&desk_id)?, own_id(&ben)?);
    assert_eq!(ann.author_of(&note)?.map(|p| p.name), Some("Ben".into()));
    // The person filter finds it through the old id and the new.
    assert!(ann.wrote_by(&note, &PersonFilter::person(&desk_id))?);
    assert!(ann.wrote_by(&note, &PersonFilter::person(&own_id(&ben)?))?);
    // On Ben's phone it is his own, under his old id too.
    let request = sync_request(&ben, &desk.version_vector()?, "Phone", "", "")?;
    let (answer, _) = answer_sync(&desk, &request, "Desk")?.ok_or("an answer")?;
    crate::lan::apply_answer(&ben, &answer)?;
    assert!(ben.wrote_by(&note, &me())?);
    assert!(ben.wrote_by(&note, &PersonFilter::person(&desk_id))?);
    Ok(())
}

#[test]
fn a_profile_naming_somebody_elses_id_as_its_alias_takes_nobodys_place() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let (_, note) = desk_with_a_note(&ann, &ben)?;
    let ben_id = own_id(&ben)?;
    let mallory = SigningKey::from_seed([9; 32]);
    let key = BASE64.encode(mallory.public_key());
    let id = "0".repeat(32);
    let record = Profile {
        id: id.clone(),
        name: "Mallory".into(),
        picture: None,
        version: 1,
    }
    .encode();
    let claim = SignedProfile {
        sig: mallory.sign(&record_bytes(&record)),
        record,
        signer: key.clone(),
        first: key.clone(),
        keys: Vec::new(),
        aliases: vec![ProfileAlias {
            alias: ben_id.clone(),
            to: id.clone(),
            by: key,
            sig: mallory.sign(&crate::profile::alias_bytes(&ben_id, &id)),
        }],
        fresh: false,
    }
    .to_json();
    let from_ben = ann.people_from(&ben.device_id()?)?;
    assert_eq!(
        ann.take_person(&claim, Some(&from_ben))?,
        PersonTaken::Refused
    );
    assert_eq!(ann.take_person(&claim, None)?, PersonTaken::Added);
    assert_eq!(ann.resolve_person(&ben_id)?, ben_id);
    assert_eq!(ann.person_shown(&ben_id)?.name, "Ben");
    assert_eq!(ann.author_of(&note)?.map(|p| p.name), Some("Ben".into()));
    Ok(())
}

/// Ben's phone syncs with Ann's desk, as people sharing an Area.
fn sync(ben: &Store, ann: &Store) -> TestResult {
    let request = sync_request(ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
    let (answer, _) = answer_sync(ann, &request, "Desk")?.ok_or("an answer")?;
    apply_answer_as(ben, &answer, true, None)?;
    Ok(())
}

/// The Note `from` sends, accepted on `to`.
fn accepted(to: &Store, file: &[u8]) -> std::result::Result<String, Box<dyn std::error::Error>> {
    let opened = to.open_sent_item(file)?.map_err(|_| "a Sent Item")?;
    Ok(to.accept_sent_item(&opened.item, false)?.ok_or("a Note")?)
}

fn sent_from(from: &Store) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    let note = Notes::new(from).capture("Hi", None, None)?;
    Ok(from.send_note(&note.id, None, false)?)
}

/// A `received` link as accepting a Sent Item writes it.
fn link(key: &str, person: &str) -> String {
    json!({"from": key, "id": "x", "profile": person, "version": 1}).to_string()
}

fn key_of(store: &Store) -> std::result::Result<String, Box<dyn std::error::Error>> {
    Ok(BASE64.encode(store.signing_key()?.public_key()))
}

#[test]
fn a_received_link_another_persons_device_wrote_marks_nothing() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let cid = device(3, "Cid")?;
    let area = household(&ann, &ben)?;
    let cids = accepted(&ann, &sent_from(&cid)?)?;
    let cid_id = own_id(&cid)?;
    assert_eq!(ann.author_of(&cids)?.map(|p| p.id), Some(cid_id.clone()));
    // Ben's modified app marks his Notes as Ann's and as Cid's.
    let as_ann = Notes::new(&ben).capture("Eggs", Some(&area), None)?;
    let as_cid = Notes::new(&ben).capture("Flour", Some(&area), None)?;
    let ann_link = link(&key_of(&ann)?, &own_id(&ann)?);
    ben.append(&as_ann.id, "received", Some(&ann_link), None)?;
    ben.append(
        &as_cid.id,
        "received",
        Some(&link(&key_of(&cid)?, &cid_id)),
        None,
    )?;
    sync(&ben, &ann)?;
    let ben_id = own_id(&ben)?;
    assert_eq!(
        ann.author_of(&as_ann.id)?.map(|p| p.id),
        Some(ben_id.clone())
    );
    assert!(!ann.wrote_by(&as_ann.id, &me())?);
    assert_eq!(ann.author_of(&as_cid.id)?.map(|p| p.id), Some(ben_id));
    Ok(())
}

#[test]
fn a_newer_received_link_another_persons_device_wrote_hides_nothing() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let cid = device(3, "Cid")?;
    let area = household(&ann, &ben)?;
    let cids = accepted(&ann, &sent_from(&cid)?)?;
    // A Manager can write on Ann's Note too.
    ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
    sync(&ben, &ann)?;
    Notes::new(&ann).file(&cids, &area)?;
    sync(&ben, &ann)?;
    let forged = link(&key_of(&ann)?, &own_id(&ann)?);
    // Later than anything Ann wrote: each Device has a clock of its own.
    let later = ann.now() + chrono::Duration::hours(1);
    ben.append(&cids, "received", Some(&forged), Some(later))?;
    sync(&ben, &ann)?;
    assert_eq!(ann.current(&cids, "received")?, Some(forged), "it landed");
    assert_eq!(ann.author_of(&cids)?.map(|p| p.id), Some(own_id(&cid)?));
    Ok(())
}

type Boxed<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Cid's Note "Hi" with a Reminder, accepted on `ann`: Cid's Note and
/// the Note `ann` holds it as.
fn from_cid(ann: &Store, cid: &Store) -> Boxed<(String, String)> {
    let sent = Notes::new(cid).capture("Hi", None, None)?.id;
    let schedule = crate::Schedule::new(
        crate::Day::parse("2026-03-03").ok_or("day")?,
        &[crate::ClockTime::parse("10:00").ok_or("time")?],
        None,
    )
    .ok_or("schedule")?;
    crate::Routines::new(cid).spawn(&sent, &schedule)?;
    let held = accepted(ann, &cid.send_note(&sent, None, false)?)?;
    Ok((sent, held))
}

/// The `received` link `ann` keeps on `held`, with `version`.
fn relinked(ann: &Store, held: &str, version: i64) -> Boxed<String> {
    let mut link: Value = serde_json::from_str(&ann.current(held, "received")?.ok_or("a link")?)?;
    link["version"] = json!(version);
    Ok(link.to_string())
}

fn opened(store: &Store, file: &[u8]) -> Boxed<crate::sent::Opened> {
    Ok(store.open_sent_item(file)?.map_err(|_| "a Sent Item")?)
}

fn text_of(store: &Store, note: &str) -> Boxed<String> {
    Ok(Notes::new(store).by_id(note)?.ok_or("a Note")?.text)
}

fn trashed(store: &Store, note: &str) -> Boxed<bool> {
    Ok(Notes::new(store).by_id(note)?.ok_or("a Note")?.is_trashed())
}

#[test]
fn a_sent_items_link_another_persons_device_wrote_is_no_note_it_reaches() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let cid = device(3, "Cid")?;
    let area = household(&ann, &ben)?;
    let (sent, held) = from_cid(&ann, &cid)?;
    // A Manager can write on Ann's Note too.
    ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
    sync(&ben, &ann)?;
    let rent = Notes::new(&ann)
        .capture("Pay the rent", Some(&area), None)?
        .id;
    sync(&ben, &ann)?;
    // Later than the grant: each Device has a clock of its own.
    let later = ann.now() + chrono::Duration::hours(1);
    ben.append(
        &rent,
        "received",
        Some(&relinked(&ann, &held, 1)?),
        Some(later),
    )?;
    sync(&ben, &ann)?;
    assert!(ann.current(&rent, "received")?.is_some(), "it landed");

    Notes::new(&cid).edit(&sent, "Hi again")?;
    let update = opened(&ann, &cid.send_note(&sent, None, false)?)?;
    assert_eq!(update.state, SentItemState::Update);
    assert_eq!(update.accepted.as_deref(), Some(held.as_str()));
    assert_eq!(
        ann.accept_sent_item(&update.item, false)?,
        Some(held.clone())
    );
    assert_eq!(text_of(&ann, &held)?, "Hi again");
    assert_eq!(text_of(&ann, &rent)?, "Pay the rent");

    let cancel = opened(&ann, &cid.send_note(&sent, None, true)?)?;
    assert_eq!(cancel.state, SentItemState::Cancel);
    assert_eq!(
        ann.accept_sent_item(&cancel.item, false)?,
        Some(held.clone())
    );
    assert!(trashed(&ann, &held)?);
    assert!(!trashed(&ann, &rent)?);
    Ok(())
}

#[test]
fn a_members_own_note_linked_to_a_sent_item_never_accepted_is_left_alone() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let cid = device(3, "Cid")?;
    let area = household(&ann, &ben)?;
    let sent = Notes::new(&cid).capture("Hi", None, None)?.id;
    let item = opened(&ann, &cid.send_note(&sent, None, false)?)?.item;
    let eggs = Notes::new(&ben).capture("Eggs", Some(&area), None)?.id;
    let forged = json!({
        "from": key_of(&cid)?,
        "id": item.id,
        "profile": own_id(&cid)?,
        "version": 1,
    });
    ben.append(&eggs, "received", Some(&forged.to_string()), None)?;
    sync(&ben, &ann)?;
    assert!(ann.current(&eggs, "received")?.is_some(), "it landed");

    let cancel = opened(&ann, &cid.send_note(&sent, None, true)?)?;
    assert_eq!(cancel.state, SentItemState::NothingToCancel);
    assert_eq!(ann.accept_sent_item(&cancel.item, false)?, None);
    assert!(!trashed(&ann, &eggs)?);

    let update = opened(&ann, &cid.send_note(&sent, None, false)?)?;
    assert_eq!(update.state, SentItemState::Fresh);
    assert_ne!(
        ann.accept_sent_item(&update.item, false)?,
        Some(eggs.clone())
    );
    assert_eq!(text_of(&ann, &eggs)?, "Eggs");
    Ok(())
}

#[test]
fn the_link_ones_own_phone_wrote_carries_updates_applied_on_the_desk() -> TestResult {
    let ann = device(1, "Ann")?;
    let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([4; 32])))?;
    let cid = device(3, "Cid")?;
    remember(&ann, &desk, false)?;
    remember(&desk, &ann, false)?;
    let (sent, held) = from_cid(&ann, &cid)?;
    own_sync(&desk, &ann)?;
    Notes::new(&cid).edit(&sent, "Hi again")?;
    let update = opened(&desk, &cid.send_note(&sent, None, false)?)?;
    assert_eq!(update.state, SentItemState::Update);
    assert!(update.changes.ok_or("changes")?.routines_added.is_empty());
    assert_eq!(
        desk.accept_sent_item(&update.item, false)?,
        Some(held.clone())
    );
    assert_eq!(text_of(&desk, &held)?, "Hi again");
    own_sync(&desk, &ann)?;

    let cancel = opened(&ann, &cid.send_note(&sent, None, true)?)?;
    assert_eq!(cancel.state, SentItemState::Cancel);
    assert_eq!(
        ann.accept_sent_item(&cancel.item, false)?,
        Some(held.clone())
    );
    assert!(trashed(&ann, &held)?);
    assert!(crate::Routines::new(&ann).all()?.is_empty(), "alarms off");
    Ok(())
}

#[test]
fn a_newer_link_another_persons_device_wrote_hides_no_update() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let cid = device(3, "Cid")?;
    let area = household(&ann, &ben)?;
    let (sent, held) = from_cid(&ann, &cid)?;
    ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
    sync(&ben, &ann)?;
    Notes::new(&ann).file(&held, &area)?;
    sync(&ben, &ann)?;
    let routines = crate::Routines::new(&ann).of_note(&held)?;
    let [routine] = routines.as_slice() else {
        return Err("one Reminder".into());
    };
    // Claims every version is held already, and the Reminder is none
    // of Cid's; later than anything Ann wrote.
    let later = ann.now() + chrono::Duration::hours(1);
    ben.append(
        &held,
        "received",
        Some(&relinked(&ann, &held, 99)?),
        Some(later),
    )?;
    ben.append(&routine.id, "received", Some("routine:forged"), Some(later))?;
    sync(&ben, &ann)?;
    let landed: Value = serde_json::from_str(&ann.current(&held, "received")?.ok_or("a link")?)?;
    assert_eq!(landed["version"], json!(99), "it landed");
    assert_eq!(
        ann.current(&routine.id, "received")?.as_deref(),
        Some("routine:forged")
    );

    Notes::new(&cid).edit(&sent, "Hi again")?;
    let update = opened(&ann, &cid.send_note(&sent, None, false)?)?;
    assert_eq!(update.state, SentItemState::Update);
    let changes = update.changes.clone().ok_or("changes")?;
    assert!(changes.routines_added.is_empty());
    assert!(changes.routines_removed.is_empty());
    ann.accept_sent_item(&update.item, false)?;
    assert_eq!(text_of(&ann, &held)?, "Hi again");
    assert_eq!(crate::Routines::new(&ann).of_note(&held)?.len(), 1);
    Ok(())
}

#[test]
fn a_text_another_persons_device_wrote_with_a_link_is_not_the_one_sent_last() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let cid = device(3, "Cid")?;
    let area = household(&ann, &ben)?;
    let (sent, held) = from_cid(&ann, &cid)?;
    ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
    sync(&ben, &ann)?;
    Notes::new(&ann).file(&held, &area)?;
    sync(&ben, &ann)?;
    // Both at once, as accepting writes them.
    let later = Some(ann.now() + chrono::Duration::hours(1));
    let text = crate::StoredText::plain("Hi from Ben").encode();
    ben.append(&held, crate::notes::keys::TEXT, Some(&text), later)?;
    ben.append(&held, "received", Some(&relinked(&ann, &held, 1)?), later)?;
    sync(&ben, &ann)?;
    assert_eq!(text_of(&ann, &held)?, "Hi from Ben", "it landed");

    Notes::new(&cid).edit(&sent, "Hi again")?;
    let update = opened(&ann, &cid.send_note(&sent, None, false)?)?;
    let changes = update.changes.ok_or("changes")?;
    assert_eq!(changes.text_before.as_deref(), Some("Hi from Ben"));
    assert!(changes.text_edited);
    Ok(())
}

/// The link `ann` keeps on a Note it sent.
fn sent_field(ann: &Store) -> Boxed<String> {
    Ok(crate::sent::keys::sent(&ann.device_id()?))
}

/// Ann's Note "Dentist" sent to Cid and accepted there: the Sent Item
/// and the Note Cid holds it as.
fn dentist_at(ann: &Store, cid: &Store) -> Boxed<(crate::sent::SentItem, String)> {
    let dentist = Notes::new(ann).capture("Dentist", None, None)?.id;
    let item = opened(cid, &ann.send_note(&dentist, None, false)?)?.item;
    let held = cid.accept_sent_item(&item, false)?.ok_or("a Note")?;
    Ok((item, held))
}

#[test]
fn a_sent_link_another_persons_device_wrote_sends_a_new_sent_item() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let cid = device(3, "Cid")?;
    let area = household(&ann, &ben)?;
    ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
    sync(&ben, &ann)?;
    let (dentist, held) = dentist_at(&ann, &cid)?;
    let rent = Notes::new(&ann)
        .capture("Pay the rent", Some(&area), None)?
        .id;
    sync(&ben, &ann)?;
    let forged = json!({"id": dentist.id, "version": 1}).to_string();
    let later = ann.now() + chrono::Duration::hours(1);
    ben.append(&rent, &sent_field(&ann)?, Some(&forged), Some(later))?;
    sync(&ben, &ann)?;
    assert!(
        ann.current(&rent, &sent_field(&ann)?)?.is_some(),
        "it landed"
    );
    assert!(!ann.was_sent(&rent)?);

    let cancel = opened(&cid, &ann.send_note(&rent, None, true)?)?;
    assert_ne!(cancel.item.id, dentist.id);
    assert_eq!(cancel.state, SentItemState::NothingToCancel);
    assert_eq!(cid.accept_sent_item(&cancel.item, false)?, None);
    assert!(!trashed(&cid, &held)?);

    let again = opened(&cid, &ann.send_note(&rent, None, false)?)?;
    assert_ne!(again.item.id, dentist.id);
    assert_eq!(again.state, SentItemState::Fresh);
    assert_ne!(
        cid.accept_sent_item(&again.item, false)?,
        Some(held.clone())
    );
    assert_eq!(text_of(&cid, &held)?, "Dentist");
    Ok(())
}

#[test]
fn ones_own_sent_link_makes_the_second_send_an_update_through_the_desk() -> TestResult {
    let ann = device(1, "Ann")?;
    let desk = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([4; 32])))?;
    let cid = device(3, "Cid")?;
    remember(&ann, &desk, false)?;
    remember(&desk, &ann, false)?;
    let hi = Notes::new(&ann).capture("Hi", None, None)?.id;
    let first = opened(&cid, &ann.send_note(&hi, None, false)?)?.item;
    let held = cid.accept_sent_item(&first, false)?.ok_or("a Note")?;
    own_sync(&desk, &ann)?;
    Notes::new(&desk).edit(&hi, "Hi again")?;
    own_sync(&desk, &ann)?;
    assert!(ann.was_sent(&hi)?);

    let update = opened(&cid, &ann.send_note(&hi, None, false)?)?;
    assert_eq!(update.item.id, first.id);
    assert_eq!(update.item.version, 2);
    assert_eq!(update.state, SentItemState::Update);
    assert_eq!(
        cid.accept_sent_item(&update.item, false)?,
        Some(held.clone())
    );
    assert_eq!(text_of(&cid, &held)?, "Hi again");
    // Another Device of Ann's sends under an id of its own (ADR 0015).
    assert!(!desk.was_sent(&hi)?);
    assert_ne!(desk.sent_item_of(&hi, false)?.id, first.id);
    Ok(())
}

#[test]
fn a_newer_sent_link_another_persons_device_wrote_hides_not_ones_own() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let cid = device(3, "Cid")?;
    let area = household(&ann, &ben)?;
    ann.set_role(&area, &own_id(&ben)?, AreaRole::Manager)?;
    sync(&ben, &ann)?;
    let (dentist, dentist_held) = dentist_at(&ann, &cid)?;
    let rent = Notes::new(&ann)
        .capture("Pay the rent", Some(&area), None)?
        .id;
    let first = opened(&cid, &ann.send_note(&rent, None, false)?)?.item;
    let held = cid.accept_sent_item(&first, false)?.ok_or("a Note")?;
    sync(&ben, &ann)?;
    let forged = json!({"id": dentist.id, "version": 99}).to_string();
    let later = ann.now() + chrono::Duration::hours(1);
    ben.append(&rent, &sent_field(&ann)?, Some(&forged), Some(later))?;
    sync(&ben, &ann)?;
    let landed: Value =
        serde_json::from_str(&ann.current(&rent, &sent_field(&ann)?)?.ok_or("a link")?)?;
    assert_eq!(landed["version"], json!(99), "it landed");

    let update = opened(&cid, &ann.send_note(&rent, None, false)?)?;
    assert_eq!(update.item.id, first.id);
    assert_eq!(update.item.version, 2);
    assert_eq!(update.state, SentItemState::Update);
    assert_eq!(
        cid.accept_sent_item(&update.item, false)?,
        Some(held.clone())
    );
    assert_eq!(text_of(&cid, &dentist_held)?, "Dentist");
    Ok(())
}

#[test]
fn a_sent_item_carrying_the_receivers_id_or_a_third_persons_shows_its_key() -> TestResult {
    let ben = device(2, "Ben")?;
    let eve = device(5, "Eve")?;
    let cid = device(3, "Cid")?;
    accepted(&ben, &sent_from(&eve)?)?;
    accepted(&ben, &sent_from(&cid)?)?;
    let forged_as =
        |id: &str, name: &str| -> std::result::Result<String, Box<dyn std::error::Error>> {
            let key = eve.signing_key()?;
            let record = Profile {
                id: id.into(),
                name: name.into(),
                picture: None,
                version: 9,
            }
            .encode();
            let copy = SignedProfile {
                sig: key.sign(&record_bytes(&record)),
                record,
                signer: key_of(&eve)?,
                first: key_of(&eve)?,
                keys: Vec::new(),
                aliases: Vec::new(),
                fresh: false,
            };
            let item = crate::sent::SentItem {
                id: format!("as-{name}"),
                version: 1,
                cancel: false,
                from: eve.own_key_record()?,
                profile: copy,
                text: format!("Hello from {name}"),
                text_format: crate::text_format::PLAIN.into(),
                text_version: crate::text_format::VERSION,
                sections: Vec::new(),
                routines: Vec::new(),
                lines: None,
                shopping: false,
            };
            accepted(
                &ben,
                &crate::sent::encode(&item, &key, &[], &mut |n| Ok(vec![0; n]))?,
            )
        };
    let eve_id = own_id(&eve)?;
    let as_ben = forged_as(&own_id(&ben)?, "Ben")?;
    assert_eq!(ben.author_of(&as_ben)?.map(|p| p.id), Some(eve_id.clone()));
    assert!(!ben.wrote_by(&as_ben, &me())?);
    let cid_id = own_id(&cid)?;
    let as_cid = forged_as(&cid_id, "Cid")?;
    assert_eq!(ben.author_of(&as_cid)?.map(|p| p.id), Some(eve_id));
    assert!(!ben.wrote_by(&as_cid, &PersonFilter::person(&cid_id))?);
    Ok(())
}

/// One LAN session between two of one's own Devices: `joiner` joins
/// `host`.
fn own_sync(joiner: &Store, host: &Store) -> TestResult {
    let request = sync_request(joiner, &host.version_vector()?, "Phone", "", "")?;
    let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
    crate::lan::apply_answer(joiner, &answer)?;
    Ok(())
}

#[test]
fn keep_mine_reaches_the_people_who_knew_the_profile_given_up() -> TestResult {
    for on_host in [true, false] {
        let phone = device(1, "Ada L.")?;
        let desk = device(2, "Ada")?;
        let cid = device(3, "Cid")?;
        // Cid shares an Area with the phone and keeps its Profile.
        household(&phone, &cid)?;
        let (phone_id, desk_id) = (own_id(&phone)?, own_id(&desk)?);
        assert_eq!(cid.resolve_person(&phone_id)?, phone_id);
        // The phone gives its Profile up for the computer's.
        if on_host {
            own_sync(&phone, &desk)?;
        } else {
            own_sync(&desk, &phone)?;
        }
        OwnProfile::new(&desk).answer(true)?;
        for _ in 0..3 {
            own_sync(&phone, &desk)?;
        }
        assert_eq!(own_id(&phone)?, desk_id);
        assert_eq!(phone.local_setting(ALIASES)?, desk.local_setting(ALIASES)?);
        sync(&cid, &phone)?;
        assert_eq!(
            cid.resolve_person(&phone_id)?,
            desk_id,
            "on host: {on_host}"
        );
        assert_eq!(cid.person_shown(&phone_id)?.name, "Ada");
    }
    Ok(())
}

#[test]
fn a_host_inviting_refuses_a_joiner_without_a_profile() -> TestResult {
    let ann = device(1, "Ann")?;
    let bare = Store::in_memory(ticking(), || Ok(SigningKey::from_seed([3; 32])))?;
    let house = Places::new(&ann).add_area("Household")?;
    let area = ann.share(&house.id)?;
    let invite = Invite {
        area,
        role: AreaRole::Member,
    };
    let request = sync_request_as(
        &bare,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    assert!(answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.is_none());
    // The host shows its own Profile to an inviting joiner.
    let answer = profile_answer(&ann)?;
    let shown = bare
        .sent_profile_of(answer.get("profile"), &ann.device_id()?)
        .ok_or("a Profile")?;
    assert_eq!(shown.profile.name, "Ann");
    assert_eq!(Some(shown.code), OwnProfile::new(&ann).code()?);
    assert!(
        bare.sent_profile_of(answer.get("profile"), &bare.device_id()?)
            .is_none()
    );
    Ok(())
}
