//! Automatic sync on the desktop (ADR 0003): for as long as the app
//! runs, the window open or closed into the tray, and only while some
//! paired Device syncs with this one on its own, the paired host serves
//! on every address of this computer, its port kept from run to run,
//! and its mDNS record says where it is. Paired phones sync with it
//! without the PIN, proven by their keys ([`Store::serve_paired`]); the
//! PIN stays for the first Pairing in the Sync dialogs. A paired phone of
//! the user asks its questions there too (#267): those go to the
//! [`AskDesk`], which answers them on a thread of its own while the
//! syncs go on.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oldbrainfix_core::Store;
use oldbrainfix_core::discovery::Advertiser;
use oldbrainfix_core::lan::{Host, Session};
use oldbrainfix_core::lan_ask::{self, AskDesk};

/// How often it looks whether a paired Device syncs on its own.
pub const CHECK_EVERY: Duration = Duration::from_secs(5);

/// Whether a sync of this Device runs now, on any of its threads (#225):
/// what the line under the menu bar shows. Each start and end wakes the
/// window, so the line comes and goes at once.
#[derive(Clone, Default)]
pub struct Syncing {
    running: Arc<AtomicUsize>,
    /// Since when syncs run without a pause, while any does.
    since: Arc<Mutex<Option<Instant>>>,
    wake: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Syncing {
    /// Tells `wake` whenever a sync starts or ends.
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Syncing {
        Syncing {
            running: Arc::default(),
            since: Arc::default(),
            wake: Some(Arc::new(wake)),
        }
    }

    /// Whether a sync runs now.
    pub fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst) > 0
    }

    /// Whether the sync line shows: while a sync runs, but not past
    /// `limit`. A sync that hangs, such as on a shared folder whose disk
    /// stopped answering, cannot be stopped from here; the line goes so
    /// that it does not move, and ask for frames, for good (#292).
    pub fn shows_line(&self, limit: Duration) -> bool {
        self.running()
            && self
                .since
                .lock()
                .ok()
                .and_then(|since| *since)
                .is_some_and(|since| since.elapsed() < limit)
    }

    /// Runs `work` as a sync: [`Syncing::running`] meanwhile.
    pub fn during<T>(&self, work: impl FnOnce() -> T) -> T {
        let _running = Running::start(self);
        work()
    }

    fn wake(&self) {
        if let Some(wake) = &self.wake {
            wake();
        }
    }
}

/// One sync running, until dropped, also when it panics.
struct Running<'a>(&'a Syncing);

impl<'a> Running<'a> {
    fn start(syncing: &'a Syncing) -> Running<'a> {
        if syncing.running.fetch_add(1, Ordering::SeqCst) == 0
            && let Ok(mut since) = syncing.since.lock()
        {
            *since = Some(Instant::now());
        }
        syncing.wake();
        Running(syncing)
    }
}

impl Drop for Running<'_> {
    fn drop(&mut self) {
        if self.0.running.fetch_sub(1, Ordering::SeqCst) == 1
            && let Ok(mut since) = self.0.since.lock()
        {
            *since = None;
        }
        self.0.wake();
    }
}

/// The paired host and its record, while wanted.
pub struct PairedHosting {
    host: Option<Host>,
    advertiser: Option<Advertiser>,
    /// Where the host binds; tests use the loopback.
    bind: IpAddr,
    /// Whether the record goes out; not in tests.
    advertise: bool,
    checked: Option<Instant>,
    /// Where the phone's questions go; without it, they are not offered.
    asking: Option<AskDesk>,
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
            asking: None,
            error: None,
        }
    }

    /// Answers the phone's questions with `desk`.
    pub fn set_asking(&mut self, desk: AskDesk) {
        self.asking = Some(desk);
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
    /// wanted, each answer as a sync of `syncing`. Returns the completed
    /// sessions.
    pub fn step(
        &mut self,
        store: &Store,
        name: &str,
        wait: Duration,
        syncing: &Syncing,
    ) -> Vec<Session> {
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
            if lan_ask::is_ask(&request) {
                match &self.asking {
                    Some(desk) => desk.take(request),
                    None => request.reply(oldbrainfix_core::lan::Response::status(404)),
                }
                next = host.next_request();
                continue;
            }
            match syncing.during(|| store.serve_paired(&request, name)) {
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

/// Runs the paired host until `stop`, as this Device named `name`, each
/// answer as a sync of `syncing`, the phone's questions answered by
/// `asking`; `synced` hears of every completed session. Conflicts that
/// came in are kept to be announced the next time the window looks.
pub fn run(
    store: &Store,
    name: &str,
    stop: &AtomicBool,
    syncing: &Syncing,
    asking: Option<AskDesk>,
    mut synced: impl FnMut(&Session),
) {
    let mut hosting = PairedHosting::default();
    if let Some(desk) = asking {
        hosting.set_asking(desk);
    }
    // Told once, not every few seconds while it lasts.
    let mut told: Option<String> = None;
    while !stop.load(Ordering::Relaxed) {
        for session in hosting.step(store, name, Duration::from_millis(500), syncing) {
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

#[cfg(test)]
mod tests;
