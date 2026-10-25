//! The shared folder on the desktop (ADR 0003, 0012): for as long as the
//! app runs, the window open or closed into the tray, the folder chosen
//! in the Sync view is synced every [`EVERY`] and whenever the view asks
//! for it at once. One round at a time, whoever starts it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use oldbrainfix_core::Store;
use oldbrainfix_core::folder::{FolderSyncResult, LocalFolder, folder_round, random_nonce};
use parking_lot::Mutex;

/// How often the folder is looked at.
pub const EVERY: Duration = Duration::from_secs(30);

/// One round at a time, from the thread and the window alike.
static ROUND: Mutex<()> = Mutex::new(());

/// One round through the chosen folder, after one still running; none
/// without a folder or when it could not be reached (kept as the status).
pub fn round(store: &Store) -> oldbrainfix_core::Result<Option<FolderSyncResult>> {
    let _one = ROUND.lock();
    let Some(path) = store.sync_folder()? else {
        return Ok(None);
    };
    folder_round(store, &mut LocalFolder::new(path), &mut random_nonce)
}

/// Syncs the chosen folder until `stop`, every [`EVERY`] and at once when
/// `wanted` is set; `synced` hears of every round that went through.
pub fn run(
    store: &Store,
    stop: &AtomicBool,
    wanted: &AtomicBool,
    mut synced: impl FnMut(&FolderSyncResult),
) {
    let mut last: Option<Instant> = None;
    // Told once, not every half minute while it lasts.
    let mut told: Option<String> = None;
    while !stop.load(Ordering::Relaxed) {
        let due = last.is_none_or(|at| at.elapsed() >= EVERY);
        if due || wanted.swap(false, Ordering::SeqCst) {
            last = Some(Instant::now());
            match round(store) {
                Ok(Some(result)) => {
                    if !result.conflicts.is_empty()
                        && let Err(e) = store.set_conflicts_arrived(true)
                    {
                        eprintln!("oldbrainfix: {e}");
                    }
                    synced(&result);
                    told = None;
                }
                Ok(None) => {}
                Err(e) => {
                    let e = e.to_string();
                    if told.as_ref() != Some(&e) {
                        eprintln!("oldbrainfix: {e}");
                        told = Some(e);
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
