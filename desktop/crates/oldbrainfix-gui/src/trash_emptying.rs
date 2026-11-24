//! "Empty the trash now" on the desktop (obf-1095), from the second row
//! of the Trash's list band: how much it deletes is counted beside the
//! window ([`oldbrainfix_core::Trash::to_empty`] reads every Note), once
//! for each mark of the log while the Trash shows, and the question asked
//! before it names those counts. Once answered, the trash is emptied on a
//! thread of its own, with a connection of its own to the window's
//! database, as deleting an Area is ([`crate::area_deletion`]); the page
//! Running lists it, without Abort. The core purges in small batches, so a
//! failed batch leaves what came before it deleted.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Instant;

use oldbrainfix_core::{Store, Trash};

use crate::folder_import::ImportStore;

/// How many Notes, and how many Routines and Reminders, emptying the
/// trash deletes.
pub type Counted = (usize, usize);

/// How emptying the trash ended: what it deleted, or what went wrong,
/// for the details.
pub type Emptied = Result<Counted, String>;

/// The counts read at a mark of the log.
type Read = (i64, Result<Counted, String>);

/// Counts what emptying the trash deletes now on `store`.
pub fn count(store: &Store) -> Result<Counted, String> {
    Trash::new(store)
        .to_empty()
        .map(|to| (to.notes.len(), to.routines.len()))
        .map_err(|e| e.to_string())
}

/// Empties the trash on `store`, all of it at once.
pub fn empty(store: &Store) -> Emptied {
    Trash::new(store)
        .empty()
        .map(|gone| (gone.notes.len(), gone.routines.len()))
        .map_err(|e| e.to_string())
}

/// The trash being emptied off the window's thread, until it answers.
struct Running {
    answer: Receiver<Emptied>,
    /// What it deletes, as counted when it started.
    counted: Counted,
    /// When it started, for the page Running.
    since: Instant,
}

/// The counts of what emptying the trash deletes, and the emptying
/// running.
#[derive(Default)]
pub struct TrashEmptying {
    /// The counts read last, kept until the next read is in.
    counted: Option<Counted>,
    /// The log's mark they were read at.
    seq: Option<i64>,
    /// The count running beside the window.
    reading: Option<Receiver<Read>>,
    running: Option<Running>,
}

impl TrashEmptying {
    /// The counts read last; `None` before the first is in.
    pub fn counted(&self) -> Option<Counted> {
        self.counted
    }

    /// Whether "Empty the trash now" can be clicked: the counts are in,
    /// something is to be deleted, and the trash is not being emptied.
    pub fn offered(&self) -> bool {
        self.running.is_none()
            && self
                .counted
                .is_some_and(|(notes, routines)| notes + routines > 0)
    }

    /// Whether the counts want reading at the log's mark `seq`: they
    /// were read at another, and neither a count nor the emptying runs.
    pub fn wants_read(&self, seq: i64) -> bool {
        self.seq != Some(seq) && self.reading.is_none() && self.running.is_none()
    }

    /// Keeps the counts `read` at the log's mark `seq`; a read that
    /// failed keeps none and is read again at the next mark.
    pub fn read_in(&mut self, seq: i64, read: Result<Counted, String>) -> Result<(), String> {
        self.seq = Some(seq);
        match read {
            Ok(counted) => {
                self.counted = Some(counted);
                Ok(())
            }
            Err(e) => {
                self.counted = None;
                Err(e)
            }
        }
    }

    /// Counts on a thread of its own, at the log's mark `seq`, on the
    /// store `open` hands it there; `false` where no thread was to be
    /// had.
    pub fn read_aside(&mut self, seq: i64, open: ImportStore) -> bool {
        let (sent, arrives) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("trash count".to_string())
            .spawn(move || {
                let store = open();
                drop(sent.send((seq, count(&store))));
            });
        match spawned {
            Ok(_) => {
                self.reading = Some(arrives);
                true
            }
            Err(_) => false,
        }
    }

    /// The counts from their thread, once they are in: `None` while none
    /// are, also once the thread broke off.
    pub fn read_arrived(&mut self) -> Option<Read> {
        let reading = self.reading.as_ref()?;
        match reading.try_recv() {
            Ok(read) => {
                self.reading = None;
                Some(read)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.reading = None;
                None
            }
        }
    }

    /// Whether the counts are being read beside the window.
    pub fn reading(&self) -> bool {
        self.reading.is_some()
    }

    /// Empties the trash on a thread of its own, on the store `open`
    /// hands it there; what it deletes was counted as `counted`. Not
    /// started twice.
    pub fn start(&mut self, open: ImportStore, counted: Counted) {
        if self.running.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let store: Arc<Store> = open();
            // A window gone meanwhile is nobody's problem.
            drop(tx.send(empty(&store)));
        });
        self.running = Some(Running {
            answer: rx,
            counted,
            since: Instant::now(),
        });
    }

    /// What is being deleted and since when, while the trash is being
    /// emptied.
    pub fn under_way(&self) -> Option<(Counted, Instant)> {
        self.running.as_ref().map(|r| (r.counted, r.since))
    }

    /// Whether the trash is being emptied.
    pub fn running(&self) -> bool {
        self.running.is_some()
    }

    /// Its answer, once it is in; the counts are read anew after it.
    pub fn finished(&mut self) -> Option<Emptied> {
        let running = self.running.as_ref()?;
        let answer = match running.answer.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return None,
            // The thread broke off without an answer.
            Err(TryRecvError::Disconnected) => Err("emptying the trash broke off".to_string()),
        };
        // What was counted before is gone: none until counted again.
        self.running = None;
        self.seq = None;
        self.counted = None;
        Some(answer)
    }
}
