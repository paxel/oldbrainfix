//! How paired Devices find the desktop's paired host on the network
//! (ADR 0003): an mDNS/DNS-SD service of type [`SERVICE_TYPE`] whose
//! instance and host names and only TXT entry are this Device's id. No
//! names: the computer's host name is often its owner's, and anyone on
//! the network hears the record. The phone resolves it, then checks the
//! certificate against the one it pinned before it sends anything.

use mdns_sd::{ServiceDaemon, ServiceInfo};

use crate::{Error, Result};

/// The service type, as the phone's `NsdManager` looks for
/// `_oldbrainfix._tcp`.
pub const SERVICE_TYPE: &str = "_oldbrainfix._tcp.local.";

/// The TXT key carrying the device id.
pub const ID_KEY: &str = "id";

/// The record for the host of `device_id` on `port`, on every address
/// of this machine as they come and go.
pub fn service_info(device_id: &str, port: u16) -> Result<ServiceInfo> {
    let host_name = format!("{device_id}.local.");
    ServiceInfo::new(
        SERVICE_TYPE,
        device_id,
        &host_name,
        "",
        port,
        &[(ID_KEY, device_id)][..],
    )
    .map(ServiceInfo::enable_addr_auto)
    .map_err(|e| Error::Invalid(format!("mdns: {e}")))
}

/// The record on the network for as long as this lives.
pub struct Advertiser {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Advertiser {
    /// Starts answering for the host of `device_id` on `port`.
    pub fn start(device_id: &str, port: u16) -> Result<Advertiser> {
        let info = service_info(device_id, port)?;
        let fullname = info.get_fullname().to_string();
        let daemon = ServiceDaemon::new().map_err(|e| Error::Invalid(format!("mdns: {e}")))?;
        daemon
            .register(info)
            .map_err(|e| Error::Invalid(format!("mdns: {e}")))?;
        Ok(Advertiser { daemon, fullname })
    }
}

impl Drop for Advertiser {
    /// Says goodbye on the network, so the phones forget the record.
    fn drop(&mut self) {
        if let Ok(done) = self.daemon.unregister(&self.fullname) {
            drop(done.recv_timeout(std::time::Duration::from_secs(1)));
        }
        drop(self.daemon.shutdown());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_carries_the_device_id_and_nothing_else() -> Result<()> {
        let info = service_info("3f9a0c1d2e4b5a6c", 45231)?;
        assert_eq!(info.get_type(), SERVICE_TYPE);
        assert_eq!(
            info.get_fullname(),
            "3f9a0c1d2e4b5a6c._oldbrainfix._tcp.local."
        );
        assert_eq!(info.get_hostname(), "3f9a0c1d2e4b5a6c.local.");
        assert_eq!(info.get_port(), 45231);
        assert_eq!(info.get_properties().len(), 1);
        assert_eq!(info.get_property_val_str(ID_KEY), Some("3f9a0c1d2e4b5a6c"));
        assert!(info.get_addresses().is_empty(), "filled in as they come");
        Ok(())
    }
}
