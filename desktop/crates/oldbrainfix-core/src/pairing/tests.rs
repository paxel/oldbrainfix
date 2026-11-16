use super::*;
use crate::store::tests::store;

#[test]
fn paired_devices_are_kept_updated_and_forgotten() -> Result<()> {
    let s = store();
    let at = s.now();
    s.remember_device(&PairedDevice {
        id: "desk".into(),
        name: "Desk".into(),
        fingerprint: "ab".repeat(32),
        host: Some("192.168.1.5".into()),
        port: Some(4000),
        last_sync: at,
        shared: false,
    })?;
    s.remember_device(&PairedDevice {
        id: "desk".into(),
        name: "Desk".into(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: at,
        shared: false,
    })?;
    let held = s.paired_device("desk")?;
    assert_eq!(held.as_ref().map(|d| d.port), Some(Some(4000)));
    assert_eq!(s.paired_devices()?.len(), 1);
    assert!(s.auto_sync_on("desk")?);
    assert!(s.auto_sync_wanted()?);
    s.set_auto_sync("desk", false)?;
    assert!(!s.auto_sync_on("desk")?);
    assert!(!s.auto_sync_wanted()?);
    s.forget_device("desk")?;
    assert!(s.paired_devices()?.is_empty());
    assert!(s.auto_sync_on("desk")?, "the switch goes with it");
    assert!(s.all_entries()?.is_empty());
    Ok(())
}

#[test]
fn a_joiner_keeps_the_address_it_hosted_at_with_the_same_certificate() -> Result<()> {
    let s = store();
    let device = |fingerprint: String, host: Option<&str>| PairedDevice {
        id: "pad".into(),
        name: "Pad".into(),
        fingerprint,
        host: host.map(str::to_string),
        port: host.map(|_| 4000),
        last_sync: s.now(),
        shared: false,
    };
    s.remember_device(&device("ab".repeat(32), Some("192.168.1.7")))?;
    s.remember_device(&device("ab".repeat(32), None))?;
    let held = s.paired_device("pad")?;
    assert_eq!(
        held.as_ref().and_then(|d| d.host.clone()).as_deref(),
        Some("192.168.1.7")
    );
    assert_eq!(held.and_then(|d| d.port), Some(4000));
    // Another certificate is another host: its address is not known.
    s.remember_device(&device("cd".repeat(32), None))?;
    let held = s.paired_device("pad")?;
    assert_eq!(held.as_ref().and_then(|d| d.host.clone()), None);
    assert_eq!(held.map(|d| d.fingerprint), Some("cd".repeat(32)));
    Ok(())
}

#[test]
fn an_address_by_hand_is_read_kept_locally_and_forgotten_with_the_device() -> Result<()> {
    let at = |host: &str, port: Option<u16>| {
        Some(HandAddress {
            host: host.into(),
            port,
        })
    };
    assert_eq!(
        HandAddress::parse(" Desk.Tail1234.ts.net "),
        at("desk.tail1234.ts.net", None)
    );
    assert_eq!(
        HandAddress::parse("100.101.102.103:4711"),
        at("100.101.102.103", Some(4711))
    );
    assert_eq!(HandAddress::parse("[fd7a::1]:80"), at("fd7a::1", Some(80)));
    assert_eq!(HandAddress::parse("fd7a::1"), at("fd7a::1", None));
    assert_eq!(
        HandAddress::parse("https://desk.local:4711/x"),
        at("desk.local", Some(4711))
    );
    for refused in [
        "",
        "desk:0",
        "desk:+80",
        "desk:0x50",
        "desk:65536",
        "a b",
        "ftp://desk",
        "[desk]:1",
        "me@desk",
        "desk:",
    ] {
        assert_eq!(HandAddress::parse(refused), None, "{refused}");
    }
    assert_eq!(
        at("fd7a::1", Some(80)).map(|a| a.text()).as_deref(),
        Some("[fd7a::1]:80")
    );

    let s = store();
    assert!(s.set_hand_address("desk", "Desk.ts.net:4711")?);
    assert_eq!(s.hand_address("desk")?, at("desk.ts.net", Some(4711)));
    assert!(!s.set_hand_address("desk", "desk:99999")?, "refused");
    assert_eq!(
        s.hand_address("desk")?,
        at("desk.ts.net", Some(4711)),
        "kept"
    );
    assert!(s.all_entries()?.is_empty(), "never synced");
    s.forget_device("desk")?;
    assert_eq!(s.hand_address("desk")?, None);
    assert!(s.set_hand_address("desk", "desk.ts.net")?);
    assert!(s.set_hand_address("desk", "  ")?);
    assert_eq!(s.hand_address("desk")?, None, "emptied");
    Ok(())
}

#[test]
fn conflicts_arrived_are_kept_until_announced() -> Result<()> {
    let s = store();
    assert!(!s.conflicts_arrived()?);
    s.set_conflicts_arrived(true)?;
    assert!(s.conflicts_arrived()?);
    s.set_conflicts_arrived(false)?;
    assert!(!s.conflicts_arrived()?);
    Ok(())
}
