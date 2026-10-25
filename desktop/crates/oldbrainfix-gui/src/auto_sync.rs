//! Automatic sync on the desktop (ADR 0003): for as long as the app
//! runs, the window open or closed into the tray, and only while some
//! paired Device syncs with this one on its own, the paired host serves
//! on every address of this computer, its port kept from run to run,
//! and its mDNS record says where it is. Paired phones sync with it
//! without the PIN, proven by their keys ([`Store::serve_paired`]); the
//! PIN stays for the first Pairing in the Sync view.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use oldbrainfix_core::Store;
use oldbrainfix_core::discovery::Advertiser;
use oldbrainfix_core::lan::{Host, Session};

/// How often it looks whether a paired Device syncs on its own.
pub const CHECK_EVERY: Duration = Duration::from_secs(5);

/// The paired host and its record, while wanted.
pub struct PairedHosting {
    host: Option<Host>,
    advertiser: Option<Advertiser>,
    /// Where the host binds; tests use the loopback.
    bind: IpAddr,
    /// Whether the record goes out; not in tests.
    advertise: bool,
    checked: Option<Instant>,
    /// What went wrong last, for the log.
    pub error: Option<String>,
}

impl Default for PairedHosting {
    fn default() -> Self {
        PairedHosting::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), true)
    }
}

impl PairedHosting {
    pub fn new(bind: IpAddr, advertise: bool) -> PairedHosting {
        PairedHosting {
            host: None,
            advertiser: None,
            bind,
            advertise,
            checked: None,
            error: None,
        }
    }

    pub fn hosting(&self) -> bool {
        self.host.is_some()
    }

    pub fn port(&self) -> Option<u16> {
        self.host.as_ref().map(Host::port)
    }

    /// Starts or stops the host as the paired Devices want it.
    pub fn check(&mut self, store: &Store) {
        self.checked = Some(Instant::now());
        let wanted = match store.auto_sync_wanted() {
            Ok(wanted) => wanted,
            Err(e) => {
                self.error = Some(e.to_string());
                return;
            }
        };
        if !wanted {
            self.advertiser = None;
            self.host = None;
            return;
        }
        if self.host.is_none() {
            match store.start_paired_host(self.bind) {
                Ok(host) => self.host = Some(host),
                Err(e) => {
                    self.error = Some(e.to_string());
                    return;
                }
            }
        }
        if self.advertise
            && self.advertiser.is_none()
            && let Some(port) = self.port()
        {
            match store
                .device_id()
                .and_then(|id| Advertiser::start(&id, port))
            {
                Ok(advertiser) => self.advertiser = Some(advertiser),
                Err(e) => self.error = Some(e.to_string()),
            }
        }
    }

    /// Answers what came in for up to `wait`, as this Device named
    /// `name`, looking again every [`CHECK_EVERY`] whether it is still
    /// wanted. Returns the completed sessions.
    pub fn step(&mut self, store: &Store, name: &str, wait: Duration) -> Vec<Session> {
        if self.checked.is_none_or(|at| at.elapsed() >= CHECK_EVERY) {
            self.check(store);
        }
        let Some(host) = &self.host else {
            std::thread::sleep(wait);
            return Vec::new();
        };
        let mut sessions = Vec::new();
        let mut next = host.wait_request(wait);
        while let Some(request) = next {
            match store.serve_paired(&request, name) {
                Ok((response, session)) => {
                    request.reply(response);
                    sessions.extend(session);
                }
                Err(e) => {
                    request.reply(oldbrainfix_core::lan::Response::status(500));
                    self.error = Some(e.to_string());
                }
            }
            next = host.next_request();
        }
        sessions
    }
}

/// Runs the paired host until `stop`, as this Device named `name`;
/// `synced` hears of every completed session. Conflicts that came in
/// are kept to be announced the next time the window looks.
pub fn run(store: &Store, name: &str, stop: &AtomicBool, mut synced: impl FnMut(&Session)) {
    let mut hosting = PairedHosting::default();
    // Told once, not every few seconds while it lasts.
    let mut told: Option<String> = None;
    while !stop.load(Ordering::Relaxed) {
        for session in hosting.step(store, name, Duration::from_millis(500)) {
            if !session.conflicts.is_empty()
                && let Err(e) = store.set_conflicts_arrived(true)
            {
                eprintln!("oldbrainfix: {e}");
            }
            synced(&session);
        }
        if let Some(e) = hosting.error.take()
            && told.as_ref() != Some(&e)
        {
            eprintln!("oldbrainfix: {e}");
            told = Some(e);
        }
    }
}
