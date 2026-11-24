//! Deleting an Area beside the window (obf-1093), as the phone's
//! `area_deletion.dart`: once its dialog is answered, the Area's Notes go
//! where the dialog said on a thread of its own, with a connection of its
//! own to the window's database, each Note in a transaction of its own
//! and a rest after it ([`crate::folder_import::low_priority`]), then the
//! Area with its Topics. The window reads and writes on meanwhile; the
//! Area's row says how many of its Notes have gone. Aborted on the page
//! Running (obf-1078), it stops before its next Note: the Notes gone stay
//! where they went, the Area stays with the others.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Instant;

use oldbrainfix_core::{NotesGo, Places};
use parking_lot::Mutex;

use crate::folder_import::{ImportStore, low_priority};

/// How far deleting an Area is: `None` until its Notes are counted, then
/// how many of how many have gone.
pub type Moved = Option<(usize, usize)>;

/// How deleting an Area ended: whether the Area went, `false` when it
/// was aborted first (obf-1078); what went wrong, for the details.
pub type Deleted = Result<bool, String>;

/// An Area being deleted off the window's thread, until it answers.
struct Running {
    area_id: String,
    answer: Receiver<Deleted>,
    moved: Arc<Mutex<Moved>>,
    /// When it started, for the page Running (obf-1078).
    since: Instant,
    /// Raised by Abort there: it stops before its next Note.
    stop: Arc<AtomicBool>,
}

/// An Area being deleted, as the page Running lists it (obf-1078): its
/// id, how far it is, since when, and whether it was aborted.
pub struct Deleting {
    pub area_id: String,
    pub moved: Moved,
    pub since: Instant,
    pub aborting: bool,
}

/// The Areas being deleted beside the window, in the order they were
/// started.
#[derive(Default)]
pub struct AreaDeletions {
    running: Vec<Running>,
}

impl AreaDeletions {
    /// Deletes the Area `area_id` on a thread of its own, on the store
    /// `open` hands it there, its Notes going where `notes_go` says. An
    /// Area already being deleted is not started again: `false` then.
    pub fn start(&mut self, open: ImportStore, area_id: &str, notes_go: NotesGo) -> bool {
        if self.of(area_id).is_some() {
            return false;
        }
        let moved = Arc::new(Mutex::new(None));
        let shared = moved.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let id = area_id.to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let asked = stop.clone();
        std::thread::spawn(move || {
            let store = open();
            let tell = low_priority(|done, of| *shared.lock() = Some((done, of)));
            let stopped = || asked.load(Ordering::SeqCst);
            let deleted = Places::new(&store)
                .delete_area_stopping(&id, notes_go, &tell, &stopped)
                .map_err(|e| e.to_string());
            // A window gone meanwhile is nobody's problem.
            drop(tx.send(deleted));
        });
        self.running.push(Running {
            area_id: area_id.to_string(),
            answer: rx,
            moved,
            since: Instant::now(),
            stop,
        });
        true
    }

    /// Stops deleting the Area `area_id` before its next Note (obf-1078).
    pub fn abort(&self, area_id: &str) {
        if let Some(running) = self.running.iter().find(|r| r.area_id == area_id) {
            running.stop.store(true, Ordering::SeqCst);
        }
    }

    /// The Areas being deleted, in the order they were started.
    pub fn list(&self) -> Vec<Deleting> {
        self.running
            .iter()
            .map(|r| Deleting {
                area_id: r.area_id.clone(),
                moved: *r.moved.lock(),
                since: r.since,
                aborting: r.stop.load(Ordering::SeqCst),
            })
            .collect()
    }

    /// How far deleting the Area `area_id` is; `None` while it is not
    /// being deleted.
    pub fn of(&self, area_id: &str) -> Option<Moved> {
        self.running
            .iter()
            .find(|r| r.area_id == area_id)
            .map(|r| *r.moved.lock())
    }

    /// Whether any Area is being deleted.
    pub fn running(&self) -> bool {
        !self.running.is_empty()
    }

    /// Takes in the answers there are, each with its Area's id and how
    /// far it came; the others run on.
    pub fn finished(&mut self) -> Vec<(String, Deleted, Moved)> {
        let mut done = Vec::new();
        self.running.retain(|r| {
            let answer = match r.answer.try_recv() {
                Ok(answer) => answer,
                Err(TryRecvError::Empty) => return true,
                // The thread broke off without an answer.
                Err(TryRecvError::Disconnected) => Err("deleting the Area broke off".to_string()),
            };
            done.push((r.area_id.clone(), answer, *r.moved.lock()));
            false
        });
        done
    }
}
