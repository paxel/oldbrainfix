//! The weekly Backup on the desktop (ADR 0016): for as long as the app
//! runs, the window open or closed into the tray, a Backup due is
//! written into the folder chosen in the Settings. One at a time,
//! whoever starts it: the thread or "Back up now".

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use oldbrainfix_core::Store;
use oldbrainfix_core::backup::{back_up_if_due, back_up_now, system_random};
use parking_lot::Mutex;

/// How often the thread looks whether a Backup is due.
pub const EVERY: Duration = Duration::from_secs(600);

/// One Backup at a time, from the thread and the window alike.
static ONE: Mutex<()> = Mutex::new(());

/// Writes a Backup into the chosen folder now; its name, none without a
/// folder. A failure is kept for the Settings and returned.
pub fn now(store: &Store) -> oldbrainfix_core::Result<Option<String>> {
    let _one = ONE.lock();
    let Some(folder) = store.backup_folder()? else {
        return Ok(None);
    };
    back_up_now(store, Path::new(&folder), &mut system_random).map(Some)
}

/// Writes the weekly Backup when it is due; its name, or none.
pub fn if_due(store: &Store) -> oldbrainfix_core::Result<Option<String>> {
    let _one = ONE.lock();
    back_up_if_due(store, &mut system_random)
}

/// Looks every [`EVERY`] until `stop` whether a Backup is due, and
/// writes it; `wrote` hears of each.
pub fn run(store: &Store, stop: &AtomicBool, mut wrote: impl FnMut()) {
    let mut last: Option<Instant> = None;
    // Told once, not every ten minutes while it lasts.
    let mut told: Option<String> = None;
    while !stop.load(Ordering::Relaxed) {
        if last.is_none_or(|at| at.elapsed() >= EVERY) {
            last = Some(Instant::now());
            match if_due(store) {
                Ok(Some(_)) => {
                    told = None;
                    wrote();
                }
                Ok(None) => {}
                Err(e) => {
                    let e = e.to_string();
                    if told.as_ref() != Some(&e) {
                        eprintln!("oldbrainfix: backup: {e}");
                        told = Some(e);
                    }
                    wrote();
                }
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
