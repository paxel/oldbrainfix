//! The Devices this one has synced with in person, as the Dart core's
//! `Pairing` keeps them: one local setting each, never synced (ADR 0008).

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::Result;
use crate::entry::{dart_iso, parse_time};
use crate::store::Store;

const PREFIX: &str = "paired:";
const AUTO_PREFIX: &str = "autosync:";
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

    /// Forgets the paired Device `id` and its switch; its key stays.
    pub fn forget_device(&self, id: &str) -> Result<()> {
        self.remove_local_setting(&format!("{PREFIX}{id}"))?;
        self.remove_local_setting(&format!("{AUTO_PREFIX}{id}"))
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
