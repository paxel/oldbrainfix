//! The Devices this one has synced with in person, as the Dart core's
//! `Pairing` keeps them: one local setting each, never synced (ADR 0008).

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::entry::{dart_iso, parse_time};
use crate::store::Store;
use crate::{Error, Result};

const PREFIX: &str = "paired:";
const AUTO_PREFIX: &str = "autosync:";
const HELD_PREFIX: &str = "held:";
const ADDRESS_PREFIX: &str = "address:";
const CONFLICTS_ARRIVED: &str = "sync:conflictsArrived";

/// A paired Device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedDevice {
    pub id: String,
    pub name: String,
    /// Its hosting certificate's SHA-256 fingerprint, hex; empty for a
    /// Device that only ever joined.
    pub fingerprint: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub last_sync: DateTime<Utc>,
    /// Whether it is another person's Device, paired by invite to a
    /// Shared Area: it gets only what it shares (ADR 0013). Once one,
    /// always one.
    pub shared: bool,
}

impl PairedDevice {
    pub fn to_json(&self) -> Value {
        let mut json = json!({
            "id": self.id,
            "name": self.name,
            "fingerprint": self.fingerprint,
            "host": self.host,
            "port": self.port,
            "lastSync": dart_iso(self.last_sync),
        });
        if self.shared
            && let Some(map) = json.as_object_mut()
        {
            map.insert("shared".to_string(), Value::Bool(true));
        }
        json
    }

    pub fn from_json(json: &Value) -> Option<PairedDevice> {
        Some(PairedDevice {
            id: json.get("id")?.as_str()?.to_string(),
            name: json.get("name")?.as_str()?.to_string(),
            fingerprint: json.get("fingerprint")?.as_str()?.to_string(),
            host: json
                .get("host")
                .and_then(|h| h.as_str())
                .map(str::to_string),
            port: json
                .get("port")
                .and_then(|p| p.as_u64())
                .and_then(|p| u16::try_from(p).ok()),
            last_sync: parse_time(json.get("lastSync")?.as_str()?).ok()?,
            shared: json.get("shared").and_then(Value::as_bool) == Some(true),
        })
    }
}

/// An address typed by hand for a paired Device (#267, ADR 0003), such
/// as a Tailscale name or an address of a VPN, where it is tried when
/// mDNS finds nothing; its port, when none is typed, is the one the
/// Device was last found at. The pinned certificate still decides
/// whether the Device answering there is the one paired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandAddress {
    /// A name or an IPv4 address in lower case, or an IPv6 address
    /// without its brackets.
    pub host: String,
    pub port: Option<u16>,
}

impl HandAddress {
    /// The address `typed` names, as the Dart core's
    /// `HandAddress.parse`: a host, `host:port`, `[v6]` or `[v6]:port`,
    /// or a bare IPv6 address; a pasted `http://` or `https://` and a
    /// path after the host are dropped. None for anything else, a port
    /// that is not 1 to 65535 in digits included.
    pub fn parse(typed: &str) -> Option<HandAddress> {
        let text = typed.trim();
        let lower = text.to_ascii_lowercase();
        let text = if lower.starts_with("https://") {
            text.get("https://".len()..)?
        } else if lower.starts_with("http://") {
            text.get("http://".len()..)?
        } else {
            text
        };
        let text = text
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let v6 = |host: &str| {
            host.contains(':')
                && host
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
        };
        let (host, port) = if let Some(inner) = text.strip_prefix('[') {
            let (host, rest) = inner.split_once(']')?;
            if !v6(host) {
                return None;
            }
            let port = match rest {
                "" => None,
                rest => Some(port_of(rest.strip_prefix(':')?)?),
            };
            (host.to_string(), port)
        } else if text.matches(':').count() > 1 {
            if !v6(&text) {
                return None;
            }
            (text.clone(), None)
        } else {
            let (host, port) = match text.split_once(':') {
                Some((host, port)) => (host, Some(port_of(port)?)),
                None => (text.as_str(), None),
            };
            let name = !host.is_empty()
                && host
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
            if !name {
                return None;
            }
            (host.to_string(), port)
        };
        Some(HandAddress { host, port })
    }

    /// The address as it is kept and shown: an IPv6 address in brackets.
    pub fn text(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        match self.port {
            Some(port) => format!("{host}:{port}"),
            None => host,
        }
    }
}

/// A port in digits only, 1 to 65535: no sign, no `0x`.
fn port_of(text: &str) -> Option<u16> {
    if text.is_empty() || text.len() > 5 || !text.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    text.parse().ok().filter(|port| *port > 0)
}

impl Store {
    /// The address typed by hand for the paired Device `id`, if one is.
    pub fn hand_address(&self, id: &str) -> Result<Option<HandAddress>> {
        Ok(self
            .local_setting(&format!("{ADDRESS_PREFIX}{id}"))?
            .and_then(|kept| HandAddress::parse(&kept)))
    }

    /// Keeps `typed` as the address by hand of the paired Device `id`, a
    /// local setting of this Device, never synced; an empty one removes
    /// it. Whether it was taken: an address [`HandAddress::parse`]
    /// refuses changes nothing.
    pub fn set_hand_address(&self, id: &str, typed: &str) -> Result<bool> {
        let key = format!("{ADDRESS_PREFIX}{id}");
        if typed.trim().is_empty() {
            self.remove_local_setting(&key)?;
            return Ok(true);
        }
        let Some(address) = HandAddress::parse(typed) else {
            return Ok(false);
        };
        self.set_local_setting(&key, &address.text())?;
        Ok(true)
    }

    /// Every paired Device, by name.
    pub fn paired_devices(&self) -> Result<Vec<PairedDevice>> {
        let mut devices: Vec<PairedDevice> = self
            .local_settings_with_prefix(PREFIX)?
            .into_iter()
            .filter_map(|(_, raw)| serde_json::from_str(&raw).ok())
            .filter_map(|json| PairedDevice::from_json(&json))
            .collect();
        devices.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(devices)
    }

    /// The paired Device `id`, if it is one.
    pub fn paired_device(&self, id: &str) -> Result<Option<PairedDevice>> {
        Ok(self
            .local_setting(&format!("{PREFIX}{id}"))?
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .and_then(|json| PairedDevice::from_json(&json)))
    }

    /// Remembers `device`, as the Dart core's `rememberDevice`: a Device
    /// that only joined keeps the fingerprint it had when it hosted, and
    /// a Device whose address is not known now the address it had with
    /// that fingerprint; one paired by invite stays shared.
    pub fn remember_device(&self, device: &PairedDevice) -> Result<()> {
        let mut kept = device.clone();
        if let Some(held) = self.paired_device(&device.id)? {
            kept.shared |= held.shared;
            if device.fingerprint.is_empty() {
                kept.fingerprint = held.fingerprint.clone();
            }
            if kept.fingerprint == held.fingerprint && device.host.is_none() {
                kept.host = held.host;
                kept.port = held.port;
            }
        }
        self.set_local_setting(
            &format!("{PREFIX}{}", device.id),
            &kept.to_json().to_string(),
        )
    }

    /// Forgets the paired Device `id`, its switch and its address by
    /// hand; its signing key stays.
    pub fn forget_device(&self, id: &str) -> Result<()> {
        self.remove_local_setting(&format!("{PREFIX}{id}"))?;
        self.remove_local_setting(&format!("{AUTO_PREFIX}{id}"))?;
        self.remove_local_setting(&format!("{ADDRESS_PREFIX}{id}"))?;
        self.remove_local_setting(&format!("{HELD_PREFIX}{id}"))
    }

    /// The version vector the Device `id` last said it holds, in a LAN
    /// session or a bundle it wrote, as the Dart core's `heldBy`; empty
    /// when it never said.
    pub fn held_by(&self, id: &str) -> Result<HashMap<String, i64>> {
        Ok(self
            .local_setting(&format!("{HELD_PREFIX}{id}"))?
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .and_then(|json| {
                json.as_object().map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| v.as_i64().map(|n| (k.clone(), n)))
                        .collect()
                })
            })
            .unwrap_or_default())
    }

    /// The Device `id` said it holds `vector`: kept, each counter only
    /// ever growing.
    pub fn note_held_by(&self, id: &str, vector: &HashMap<String, i64>) -> Result<()> {
        if id.is_empty() || id == self.device_id()? {
            return Ok(());
        }
        let mut held: BTreeMap<String, i64> = self.held_by(id)?.into_iter().collect();
        for (device, n) in vector {
            let at = held.entry(device.clone()).or_insert(0);
            if *n > *at {
                *at = *n;
            }
        }
        let json = serde_json::to_string(&held).map_err(|e| Error::Invalid(e.to_string()))?;
        self.set_local_setting(&format!("{HELD_PREFIX}{id}"), &json)
    }

    /// Whether this Device syncs with the paired Device `id` on its own;
    /// on unless switched off, as the Dart core's `autoSyncOn`. Kept apart
    /// from the [`PairedDevice`], which every sync writes anew.
    pub fn auto_sync_on(&self, id: &str) -> Result<bool> {
        Ok(self
            .local_setting(&format!("{AUTO_PREFIX}{id}"))?
            .as_deref()
            != Some("off"))
    }

    pub fn set_auto_sync(&self, id: &str, on: bool) -> Result<()> {
        let key = format!("{AUTO_PREFIX}{id}");
        if on {
            self.remove_local_setting(&key)
        } else {
            self.set_local_setting(&key, "off")
        }
    }

    /// Whether any paired Device syncs with this one on its own.
    pub fn auto_sync_wanted(&self) -> Result<bool> {
        for device in self.paired_devices()? {
            if self.auto_sync_on(&device.id)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether a sync nobody watched brought Conflicts not announced yet,
    /// as the Dart core's `conflictsArrived`.
    pub fn conflicts_arrived(&self) -> Result<bool> {
        Ok(self.local_setting(CONFLICTS_ARRIVED)?.is_some())
    }

    pub fn set_conflicts_arrived(&self, arrived: bool) -> Result<()> {
        if arrived {
            self.set_local_setting(CONFLICTS_ARRIVED, "1")
        } else {
            self.remove_local_setting(CONFLICTS_ARRIVED)
        }
    }
}

#[cfg(test)]
mod tests {
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
}
