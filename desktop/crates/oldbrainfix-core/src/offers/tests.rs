use super::*;
use crate::bundle::{
    import_bundle, write_area_offer_now, write_bundle_now, write_person_bundle_now,
};
use crate::folder::{MemoryFolder, folder_sync, random_nonce, write_sealed};
use crate::lan::{answer_sync, apply_answer, sync_request};
use crate::profile::OwnProfile;
use crate::routines::Routines;
use crate::schedule::{ClockTime, Day, Schedule};
use crate::shared::AreaAccess;
use crate::store::tests::ticking;
use crate::{Notes, Places};
use std::path::{Path, PathBuf};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// One clock for every Device of a test, a second further at each
/// reading, so what was written later wins everywhere.
static TICKS: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

fn bare(seed: u8, name: &str) -> Made<Store> {
    let start = ticking()();
    let clock: crate::Clock = Box::new(move || {
        let n = TICKS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        start + chrono::Duration::seconds(n)
    });
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

fn device(seed: u8, name: &str) -> Made<Store> {
    let store = bare(seed, name)?;
    OwnProfile::new(&store).rename(name)?;
    Ok(store)
}

fn id_of(store: &Store) -> Made<String> {
    Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
}

/// Each sent the other their Profile card through a messenger.
fn met(a: &Store, b: &Store) -> TestResult {
    b.open_profile_card(&a.profile_card(false)?.ok_or("a card")?, false)?;
    a.open_profile_card(&b.profile_card(true)?.ok_or("a card")?, false)?;
    Ok(())
}

/// One LAN session between two of one's own Devices.
fn meet(joiner: &Store, host: &Store) -> TestResult {
    let request = sync_request(joiner, &host.version_vector()?, "Phone", "", "")?;
    let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
    apply_answer(joiner, &answer)?;
    Ok(())
}

fn photo() -> Vec<u8> {
    let mut bytes = vec![0xff, 0xd8];
    bytes.extend((0..64u32).map(|i| ((i * 3) & 0xff) as u8));
    bytes.extend([0xff, 0xd9]);
    bytes
}

fn texts(store: &Store) -> Made<Vec<String>> {
    let mut texts: Vec<String> = Notes::new(store)
        .all()?
        .into_iter()
        .map(|n| n.text)
        .collect();
    texts.sort();
    Ok(texts)
}

struct Offered {
    ann: Store,
    ben: Store,
    area: String,
    note: String,
    file: PathBuf,
    dir: tempfile::TempDir,
}

/// Ann's Household with a Note and its photo, a Routine and a private
/// Note beside it, shared with Ben, whom she knows from his card, as
/// `role`; the offer written.
fn offered(role: AreaRole, transport: Transport) -> Made<Offered> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    met(&ann, &ben)?;
    let house = Places::new(&ann).add_area("Household")?;
    let milk = Notes::new(&ann).capture_with("Buy milk", Some(&house.id), None, &[photo()], &[])?;
    let eight: Vec<ClockTime> = ClockTime::new(8, 0).into_iter().collect();
    let schedule = Schedule::new(Day::new(2026, 3, 2), &eight, None).ok_or("a schedule")?;
    Routines::new(&ann).spawn(&milk.id, &schedule)?;
    Notes::new(&ann).capture("Private", None, None)?;
    let area = ann.share_area_with(&house.id, &id_of(&ben)?, role, transport)?;
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("offer.obfsync");
    write_area_offer_now(&ann, &file, &area, &id_of(&ben)?)?;
    Ok(Offered {
        ann,
        ben,
        area,
        note: milk.id,
        file,
        dir,
    })
}

fn held(store: &Store, file: &Path) -> Made<HeldAreaOffer> {
    Ok(import_bundle(store, file)?.offer.ok_or("an offer")?)
}

fn is(result: Result<impl std::fmt::Debug>, want: OfferError) -> bool {
    matches!(result, Err(Error::Offer(e)) if e == want)
}

#[test]
fn the_offer_is_held_until_accepted_and_nothing_of_the_area_is_kept_before() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    let before = o.ben.all_entries()?.len();
    let got = import_bundle(&o.ben, &o.file)?;
    assert!(got.applied.is_empty());
    let offer = got.offer.ok_or("an offer")?;
    assert_eq!(offer.sender.name, "Ann");
    assert_eq!(offer.area_name, "Household");
    assert_eq!(offer.role, AreaRole::Member);
    assert_eq!(offer.trust, SenderTrust::Known);
    assert_eq!(offer.transport, Transport::Bundle);
    assert_eq!(o.ben.all_entries()?.len(), before);
    assert!(Notes::new(&o.ben).by_id(&o.note)?.is_none());
    assert!(o.ben.writing_key(&o.area)?.is_none());
    assert_eq!(held(&o.ben, &o.file)?.id, offer.id);

    assert_eq!(o.ben.accept_area_offer(&offer.id, false)?, o.area);
    assert!(o.ben.held_area_offers()?.is_empty());
    assert_eq!(texts(&o.ben)?, vec!["Buy milk"]);
    let hash = Notes::new(&o.ben)
        .by_id(&o.note)?
        .ok_or("the Note")?
        .photos
        .first()
        .cloned()
        .ok_or("a photo")?;
    assert_eq!(o.ben.photo(&hash), o.ann.photo(&hash));
    assert_eq!(Routines::new(&o.ben).of_note(&o.note)?.len(), 1);
    assert_eq!(o.ben.access_in(&o.area, None)?, AreaAccess::Member);
    assert_eq!(
        o.ben.writing_key(&o.area)?.map(|k| k.id()),
        o.ann.writing_key(&o.area)?.map(|k| k.id())
    );
    assert_eq!(
        o.ben.transport_with(&id_of(&o.ann)?)?,
        Some(Transport::Bundle)
    );
    Ok(())
}

#[test]
fn changes_travel_by_bundles_both_ways_and_the_roles_hold() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    let offer = held(&o.ben, &o.file)?;
    o.ben.accept_area_offer(&offer.id, false)?;
    Notes::new(&o.ben).capture("From Ben", Some(&o.area), None)?;
    // A Member may not change Ann's Note: her Devices ignore it.
    o.ben
        .append(&o.note, note_keys::TEXT, Some("Changed by Ben"), None)?;
    let back = o.dir.path().join("back.obfsync");
    assert!(write_person_bundle_now(&o.ben, &back, &id_of(&o.ann)?)?.is_some());
    assert_eq!(import_bundle(&o.ann, &back)?.unreadable, 0);
    assert_eq!(texts(&o.ann)?, vec!["Buy milk", "From Ben", "Private"]);

    Notes::new(&o.ann).capture("From Ann", Some(&o.area), None)?;
    let again = o.dir.path().join("again.obfsync");
    assert!(write_person_bundle_now(&o.ann, &again, &id_of(&o.ben)?)?.is_some());
    import_bundle(&o.ben, &again)?;
    assert_eq!(texts(&o.ben)?, vec!["Buy milk", "From Ann", "From Ben"]);
    Ok(())
}

#[test]
fn through_a_shared_folder_every_device_of_the_person_is_checked() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Folder)?;
    let offer = held(&o.ben, &o.file)?;
    assert_eq!(offer.transport, Transport::Folder);
    o.ben.accept_area_offer(&offer.id, false)?;
    // Ben's computer, paired as his own, takes his role over.
    let desk = bare(3, "Ben")?;
    meet(&desk, &o.ben)?;
    meet(&desk, &o.ben)?;
    assert_eq!(desk.access_in(&o.area, None)?, AreaAccess::Member);

    let mut folder = MemoryFolder::default();
    Notes::new(&desk).capture("From the desk", Some(&o.area), None)?;
    folder_sync(&o.ann, &mut folder, &mut random_nonce)?;
    folder_sync(&desk, &mut folder, &mut random_nonce)?;
    folder_sync(&o.ben, &mut folder, &mut random_nonce)?;
    Notes::new(&o.ann).capture("From Ann", Some(&o.area), None)?;
    folder_sync(&o.ann, &mut folder, &mut random_nonce)?;
    folder_sync(&o.ben, &mut folder, &mut random_nonce)?;
    // Ann met Ben's computer nowhere: its key came with the Area.
    assert_eq!(
        texts(&o.ann)?,
        vec!["Buy milk", "From Ann", "From the desk", "Private"]
    );
    assert_eq!(
        texts(&o.ben)?,
        vec!["Buy milk", "From Ann", "From the desk"]
    );
    Ok(())
}

#[test]
fn a_bundle_for_a_device_met_from_afar_holds_only_what_it_shares() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    o.ben.accept_area_offer(&held(&o.ben, &o.file)?.id, false)?;
    Notes::new(&o.ann).capture("Still private", None, None)?;
    let file = o.dir.path().join("for-ben.obfsync");
    assert!(write_bundle_now(&o.ann, &file, Some(&o.ben.device_id()?))?.is_some());
    // Nothing in it is sealed with a key Ben lacks.
    assert_eq!(import_bundle(&o.ben, &file)?.unreadable, 0);
    assert_eq!(texts(&o.ben)?, vec!["Buy milk"]);
    Ok(())
}

#[test]
fn declining_keeps_nothing_no_entry_key_profile_or_photo() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    let settings = o.ben.local_settings_with_prefix("")?;
    let entries = o.ben.all_entries()?.len();
    let offer = held(&o.ben, &o.file)?;
    let hash = Notes::new(&o.ann)
        .by_id(&o.note)?
        .ok_or("the Note")?
        .photos
        .first()
        .cloned()
        .ok_or("a photo")?;
    assert!(o.ben.photo(&hash).is_some());
    assert!(o.ben.decline_area_offer(&offer.id)?);
    assert!(o.ben.held_area_offers()?.is_empty());
    assert_eq!(o.ben.all_entries()?.len(), entries);
    assert_eq!(o.ben.local_settings_with_prefix("")?, settings);
    assert!(o.ben.photo(&hash).is_none());
    assert!(o.ben.pending_sets()?.is_empty());
    Ok(())
}

#[test]
fn decline_and_block_blocks_the_sharer_keeps_their_name_and_nothing_of_the_area() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    let offer = held(&o.ben, &o.file)?;
    let hash = Notes::new(&o.ann)
        .by_id(&o.note)?
        .ok_or("the Note")?
        .photos
        .first()
        .cloned()
        .ok_or("a photo")?;
    assert!(o.ben.block_area_offer_sender(&offer.id, false)?);
    assert!(o.ben.is_blocked(&id_of(&o.ann)?)?);
    let names: Vec<String> = o
        .ben
        .blocked_people()?
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, vec!["Ann"]);
    assert!(o.ben.pending_sets()?.is_empty());
    assert!(o.ben.photo(&hash).is_none());
    assert!(Notes::new(&o.ben).by_id(&o.note)?.is_none());
    Ok(())
}

#[test]
fn a_blocked_persons_offer_is_refused_before_its_preview_and_one_held_goes() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    let offer = held(&o.ben, &o.file)?;
    o.ben.block_person(&id_of(&o.ann)?, false)?;
    assert!(o.ben.held_area_offer(&offer.id)?.is_none());
    assert!(o.ben.pending_sets()?.is_empty());
    assert!(is(import_bundle(&o.ben, &o.file), OfferError::FromBlocked));
    assert!(o.ben.pending_sets()?.is_empty());
    Ok(())
}

#[test]
fn an_offer_of_an_area_held_here_as_its_own_is_refused() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    o.ben
        .append(&o.area, place_keys::TYPE, Some(place_keys::TYPE_AREA), None)?;
    assert!(is(import_bundle(&o.ben, &o.file), OfferError::Refused));
    assert!(o.ben.pending_sets()?.is_empty());
    Ok(())
}

#[test]
fn an_offer_sealed_for_somebody_else_is_not_for_us() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    let cat = device(4, "Cat")?;
    assert!(is(import_bundle(&cat, &o.file), OfferError::NotForUs));
    assert!(cat.pending_sets()?.is_empty());
    Ok(())
}

/// An offer of `area` from `from` to `to`, written by hand around
/// `from`'s history of it, and the bundle's files.
fn by_hand(from: &Store, area: &str, to: &Store) -> Made<(MemoryFolder, Vec<u8>)> {
    let mut folder = MemoryFolder::default();
    write_sealed(
        from,
        &mut folder,
        &from.area_history(area)?,
        &BTreeSet::new(),
        &mut random_nonce,
    )?;
    let key = to.own_key_record()?.verified_box_key().ok_or("a box key")?;
    let bytes = encode_area_offer(
        &from.area_offer_for(area, &id_of(to)?)?,
        &from.signing_key()?,
        &[key],
        &mut random_vec,
    )?;
    Ok((folder, bytes))
}

#[test]
fn an_offer_that_grants_this_device_nothing_is_not_for_us() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    let cat = device(4, "Cat")?;
    met(&o.ann, &cat)?;
    // Sealed for Cat, but the grants name only Ben.
    let (folder, bytes) = by_hand(&o.ann, &o.area, &cat)?;
    assert_eq!(
        cat.receive_area_offer(&folder, &bytes)?,
        Err(OfferError::NotForUs)
    );
    assert!(cat.pending_sets()?.is_empty());
    Ok(())
}

#[test]
fn the_preview_names_the_area_as_its_managers_do() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    o.ben.accept_area_offer(&held(&o.ben, &o.file)?.id, false)?;
    // Ben, a Member, renames the Area: relayed, but it counts nowhere.
    o.ben
        .append(&o.area, place_keys::NAME, Some("Ben's"), None)?;
    let back = o.dir.path().join("back.obfsync");
    write_person_bundle_now(&o.ben, &back, &id_of(&o.ann)?)?;
    import_bundle(&o.ann, &back)?;
    let cat = device(4, "Cat")?;
    met(&o.ann, &cat)?;
    o.ann
        .share_area_with(&o.area, &id_of(&cat)?, AreaRole::Member, Transport::Bundle)?;
    let file = o.dir.path().join("cat.obfsync");
    write_area_offer_now(&o.ann, &file, &o.area, &id_of(&cat)?)?;
    assert_eq!(held(&cat, &file)?.area_name, "Household");
    Ok(())
}

#[test]
fn an_offer_only_a_manager_signed_counts() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    let cat = device(4, "Cat")?;
    met(&o.ann, &cat)?;
    o.ann
        .share_area_with(&o.area, &id_of(&cat)?, AreaRole::Member, Transport::Bundle)?;
    let file = o.dir.path().join("again.obfsync");
    write_area_offer_now(&o.ann, &file, &o.area, &id_of(&o.ben)?)?;
    o.ben.accept_area_offer(&held(&o.ben, &file)?.id, false)?;
    // Ben, a Member, passes the Area on to Cat in his own name: her
    // grant is real, but he may not offer.
    let (folder, bytes) = by_hand(&o.ben, &o.area, &cat)?;
    assert_eq!(
        cat.receive_area_offer(&folder, &bytes)?,
        Err(OfferError::NotForUs)
    );
    assert!(cat.pending_sets()?.is_empty());
    Ok(())
}

#[test]
fn an_offer_signed_by_a_key_its_profile_does_not_bind_is_none() -> TestResult {
    let o = offered(AreaRole::Member, Transport::Bundle)?;
    let eve = SigningKey::from_seed([9; 32]);
    let offer = o.ann.area_offer_for(&o.area, &id_of(&o.ben)?)?;
    let forged = AreaOffer {
        from: KeyRecord::make(&eve),
        ..offer
    };
    let key = o
        .ben
        .own_key_record()?
        .verified_box_key()
        .ok_or("a box key")?;
    let bytes = encode_area_offer(&forged, &eve, &[key], &mut random_vec)?;
    assert_eq!(
        o.ben.receive_area_offer(&MemoryFolder::default(), &bytes)?,
        Err(OfferError::NotAnOffer)
    );
    Ok(())
}

#[test]
fn an_offer_never_moves_the_users_own_notes_into_the_area() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    met(&ann, &ben)?;
    let mine = Notes::new(&ben).capture("Ben private", None, None)?;
    let house = Places::new(&ann).add_area("Household")?;
    Notes::new(&ann).capture("Buy milk", Some(&house.id), None)?;
    // Ann's modified app files Ben's Note under her Area.
    ann.append(&mine.id, note_keys::PLACE, Some(&house.id), None)?;
    ann.append(&mine.id, note_keys::TEXT, Some("Taken"), None)?;
    ann.share_area_with(
        &house.id,
        &id_of(&ben)?,
        AreaRole::Manager,
        Transport::Bundle,
    )?;
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("offer.obfsync");
    write_area_offer_now(&ann, &file, &house.id, &id_of(&ben)?)?;
    ben.accept_area_offer(&held(&ben, &file)?.id, false)?;
    assert_eq!(ben.scope_of(&mine.id)?, crate::keys::PERSONAL_SCOPE);
    assert_eq!(
        Notes::new(&ben).by_id(&mine.id)?.ok_or("the Note")?.text,
        "Ben private"
    );
    Ok(())
}

#[test]
fn an_offer_carries_keys_of_its_area_only() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    met(&ann, &ben)?;
    let house = Places::new(&ann).add_area("Household")?;
    let offer = ann.area_offer_for(&house.id, &id_of(&ben)?)?;
    assert_eq!(
        offer
            .area_keys
            .iter()
            .map(|k| k.scope.clone())
            .collect::<Vec<_>>(),
        vec![house.id.clone()]
    );
    let personal = ann
        .writing_key(crate::keys::PERSONAL_SCOPE)?
        .ok_or("a key")?;
    let mut widened = offer.clone();
    widened.area_keys.push(personal);
    assert!(AreaOffer::parse(&widened.encode()).is_none());
    assert_eq!(
        AreaOffer::parse(&offer.encode()).map(|o| o.encode()),
        Some(offer.encode())
    );
    Ok(())
}

#[test]
fn sharing_needs_a_key_of_the_person_to_seal_for() -> TestResult {
    let ann = device(1, "Ann")?;
    let dan = device(5, "Dan")?;
    let house = Places::new(&ann).add_area("Household")?;
    ann.take_person(&dan.own_profile_for_others()?.ok_or("a Profile")?, None)?;
    assert!(
        ann.share_area_with(
            &house.id,
            &id_of(&dan)?,
            AreaRole::Member,
            Transport::Bundle
        )
        .is_err()
    );
    assert!(!ann.is_shared(&house.id)?);
    Ok(())
}
