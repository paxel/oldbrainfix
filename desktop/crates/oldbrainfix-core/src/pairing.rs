//! The Devices this one has synced with in person, as the Dart core's
//! `Pairing` keeps them: one local setting each, never synced (ADR 0008).

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::Result;
use crate::entry::{dart_iso, parse_time};
use crate::store::Store;

const PREFIX: &str = "paired:";

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
}

impl PairedDevice {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "fingerprint": self.fingerprint,
            "host": self.host,
            "port": self.port,
            "lastSync": dart_iso(self.last_sync),
        })
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
        })
    }
}

impl Store {
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

    /// Remembers `device`, keeping the fingerprint and address of a
    /// Device that hosted before when it only joined now.
    pub fn remember_device(&self, device: &PairedDevice) -> Result<()> {
        let mut kept = device.clone();
        if device.fingerprint.is_empty()
            && let Some(held) = self.paired_device(&device.id)?
        {
            kept.fingerprint = held.fingerprint;
            kept.host = held.host;
            kept.port = held.port;
        }
        self.set_local_setting(
            &format!("{PREFIX}{}", device.id),
            &kept.to_json().to_string(),
        )
    }

    /// Forgets the paired Device `id`; its key stays.
    pub fn forget_device(&self, id: &str) -> Result<()> {
        self.remove_local_setting(&format!("{PREFIX}{id}"))
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
        })?;
        s.remember_device(&PairedDevice {
            id: "desk".into(),
            name: "Desk".into(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: at,
        })?;
        let held = s.paired_device("desk")?;
        assert_eq!(held.as_ref().map(|d| d.port), Some(Some(4000)));
        assert_eq!(s.paired_devices()?.len(), 1);
        s.forget_device("desk")?;
        assert!(s.paired_devices()?.is_empty());
        assert!(s.all_entries()?.is_empty());
        Ok(())
    }
}
