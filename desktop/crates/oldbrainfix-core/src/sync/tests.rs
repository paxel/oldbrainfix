use super::*;
use crate::Notes;
use crate::signing::SigningKey;
use crate::store::tests::ticking;

fn device(seed: u8) -> Result<Store> {
    Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))
}

#[test]
fn signed_entries_pass_and_forged_ones_are_refused() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let note = Notes::new(&a).capture("Real", None, None)?;
    let mut entries = wire(&a.entries_since(&HashMap::new())?);
    let mut forged = entries[0].clone();
    forged.dseq = 99;
    forged.value = Some("forged".into());
    let mut stranger = entries[0].clone();
    stranger.device = "stranger".into();
    entries.push(forged);
    entries.push(stranger);
    let mut report = ImportReport::default();
    let applied = b.apply_entries(&entries, &[a.own_key_record()?], None, &mut report)?;
    assert_eq!(applied.len(), entries.len() - 2);
    assert_eq!(report.refused_count(), 2);
    assert_eq!(report.new_keys.len(), 1);
    let real = crate::Notes::new(&b).by_id(&note.id)?.map(|n| n.text);
    assert_eq!(real.as_deref(), Some("Real"));
    // Taken twice, nothing is new.
    let again = b.apply_entries(&entries, &[], None, &mut ImportReport::default())?;
    assert!(again.is_empty());
    // Without the key, nothing passes.
    let c = device(3)?;
    let mut report = ImportReport::default();
    assert!(
        c.apply_entries(&entries, &[], None, &mut report)?
            .is_empty()
    );
    assert_eq!(report.refused_count(), entries.len());
    Ok(())
}

#[test]
fn keys_are_valid_only_for_their_device_and_trusted_as_met() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let c = device(3)?;
    let record = a.own_key_record()?;
    assert!(record.valid());
    let claimed = KeyRecord {
        public_key: c.signing_key()?.public_key().to_vec(),
        ..record.clone()
    };
    assert!(!claimed.valid());
    let mut report = ImportReport::default();
    b.learn_keys(
        &[record.clone(), c.own_key_record()?, claimed],
        Some(&a.device_id()?),
        &mut report,
    )?;
    let trust =
        |s: &Store, d: &str| -> Result<Option<KeyTrust>> { Ok(s.pinned_key(d)?.map(|k| k.trust)) };
    assert_eq!(trust(&b, &a.device_id()?)?, Some(KeyTrust::Verified));
    assert_eq!(trust(&b, &c.device_id()?)?, Some(KeyTrust::Tofu));
    b.learn_keys(&[], Some(&c.device_id()?), &mut report)?;
    b.learn_keys(&[c.own_key_record()?], Some(&c.device_id()?), &mut report)?;
    assert_eq!(trust(&b, &c.device_id()?)?, Some(KeyTrust::Verified));
    assert_eq!(b.key_records()?.len(), 3);
    assert_eq!(KeyRecord::from_json(&record.to_json()), Some(record));
    Ok(())
}

#[test]
fn blobs_follow_the_entries_checked_by_hash() -> Result<()> {
    let a = device(1)?;
    let b = device(2)?;
    let note = Notes::new(&a).capture_with("Photo", None, None, &[b"pic".to_vec()], &[])?;
    let hash = note.photos[0].clone();
    b.apply_entries(
        &wire(&a.entries_since(&HashMap::new())?),
        &[a.own_key_record()?],
        None,
        &mut ImportReport::default(),
    )?;
    assert_eq!(b.missing_blobs()?, vec![format!("photo/{hash}")]);
    assert!(b.knows_blob(&format!("photo/{hash}"))?);
    assert!(!b.knows_blob(&format!("audio/{hash}"))?);
    assert!(!b.put_blob("photo", &hash, b"other")?);
    assert!(b.put_blob("photo", &hash, b"pic")?);
    assert!(b.missing_blobs()?.is_empty());
    Ok(())
}

#[test]
fn a_moment_is_recorded_only_when_something_changed() -> Result<()> {
    let a = device(1)?;
    assert_eq!(a.moment_for(0, false, cause::SYNC, None)?, None);
    a.append("note:a", "text", Some("x"), None)?;
    let moment = a.moment_for(0, true, cause::SYNC, Some("Phone"))?;
    assert_eq!(moment.as_ref().map(|m| m.seq), Some(0));
    assert_eq!(a.moments()?.len(), 1);
    assert_eq!(a.entries_after(0)?.len(), 1);
    assert_eq!(a.current_seq()?, 1);
    Ok(())
}
