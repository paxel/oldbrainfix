//! The vault on the desktop (#267, ADR 0050): File ▸ "Export as vault…"
//! writes every Note as a folder of Markdown files into a new folder in
//! the one picked, and the first start's "Rebuild from a vault" brings a
//! vault back into the empty app. Both run off the window's thread, one
//! at a time, and say how far they are; the core does the rest
//! ([`oldbrainfix_core::vault`]).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use oldbrainfix_core::vault::{self, Exported, Rebuilt, VaultNames, VaultProblem};
use oldbrainfix_core::{Error, Store};

use crate::app::Wall;
use crate::l10n::L10n;
use crate::names;

/// How a vault job ended.
pub enum VaultDone {
    Exported(Result<Exported, Error>),
    Rebuilt(Result<Rebuilt, Error>),
}

/// How far a job is: done, of how many.
#[derive(Default)]
struct Progress {
    done: AtomicUsize,
    total: AtomicUsize,
}

/// The export or rebuild running off the window's thread, until it
/// answers.
#[derive(Default)]
pub struct VaultJob {
    /// The answer to come, how far it is, and whether it is a rebuild.
    running: Option<(Receiver<VaultDone>, Arc<Progress>, bool)>,
}

impl VaultJob {
    /// Writes the vault of `store` into a new folder in `parent`, named in
    /// the language of `t`, dated by `wall`. Nothing while a job runs.
    pub fn export(&mut self, store: Arc<Store>, parent: PathBuf, t: L10n, wall: Wall) {
        self.run(false, move |progress| {
            let area = |a: &oldbrainfix_core::Area| names::area_name(&t, a);
            let topic = |p: &oldbrainfix_core::Topic| names::topic_name(&t, p);
            let names = VaultNames {
                area: &area,
                topic: &topic,
                unsorted: t.unsorted(),
                untitled: t.vault_untitled(),
            };
            VaultDone::Exported(vault::export(&store, &parent, &names, &*wall, progress))
        });
    }

    /// Rebuilds the vault in `folder` into the empty `store`, a file at
    /// its top into the Area `root_area`. Nothing while a job runs.
    pub fn rebuild(&mut self, store: Arc<Store>, folder: PathBuf, root_area: String) {
        self.run(true, move |progress| {
            VaultDone::Rebuilt(vault::rebuild(&store, &folder, &root_area, progress))
        });
    }

    fn run(
        &mut self,
        rebuild: bool,
        job: impl FnOnce(&dyn Fn(usize, usize)) -> VaultDone + Send + 'static,
    ) {
        if self.running.is_some() {
            return;
        }
        let progress = Arc::new(Progress::default());
        let shared = progress.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let tell = |done: usize, total: usize| {
                shared.total.store(total, Ordering::Relaxed);
                shared.done.store(done, Ordering::Relaxed);
            };
            // A window gone meanwhile is nobody's problem.
            drop(tx.send(job(&tell)));
        });
        self.running = Some((rx, progress, rebuild));
    }

    /// Whether a job runs.
    pub fn running(&self) -> bool {
        self.running.is_some()
    }

    /// How far the job running is: done, of how many; both 0 before it
    /// counted.
    pub fn progress(&self) -> (usize, usize) {
        self.running.as_ref().map_or((0, 0), |(_, p, _)| {
            (
                p.done.load(Ordering::Relaxed),
                p.total.load(Ordering::Relaxed),
            )
        })
    }

    /// The job's answer once there: `None` while it runs.
    pub fn finished(&mut self) -> Option<VaultDone> {
        let (rx, _, rebuild) = self.running.as_ref()?;
        let answer = match rx.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return None,
            // The thread broke off without an answer.
            Err(TryRecvError::Disconnected) => {
                let broke = Error::Invalid("the vault job broke off".into());
                if *rebuild {
                    VaultDone::Rebuilt(Err(broke))
                } else {
                    VaultDone::Exported(Err(broke))
                }
            }
        };
        self.running = None;
        Some(answer)
    }
}

/// What the announcement of an export says behind its tap.
pub fn exported_details(t: &L10n, exported: &Exported) -> String {
    let folder = exported.folder.display().to_string();
    let mut said = t.vault_written_details(&folder, exported.notes as i64, exported.files as i64);
    if exported.missing > 0 {
        said.push(' ');
        said.push_str(&t.vault_missing(exported.missing as i64));
    }
    said
}

/// What the announcement of a rebuild says behind its tap.
pub fn rebuilt_details(t: &L10n, rebuilt: &Rebuilt) -> String {
    let mut said = t.vault_rebuilt_details(
        rebuilt.notes as i64,
        rebuilt.elsewhere as i64,
        rebuilt.files as i64,
    );
    if rebuilt.missing > 0 {
        said.push(' ');
        said.push_str(&t.vault_rebuilt_missing(rebuilt.missing as i64));
    }
    if rebuilt.skipped > 0 {
        said.push(' ');
        said.push_str(&t.vault_rebuilt_skipped(rebuilt.skipped as i64));
    }
    said
}

/// Why a rebuild wrote nothing, and what to do.
pub fn rebuild_failure(t: &L10n, error: &Error) -> (&'static str, &'static str) {
    match error {
        Error::Vault(VaultProblem::NotEmpty) => {
            (t.vault_not_empty_cause(), t.vault_not_empty_fix())
        }
        Error::Vault(VaultProblem::NotAVault) => (t.vault_no_vault_cause(), t.vault_no_vault_fix()),
        Error::Vault(VaultProblem::NeedsUpdate) => {
            (t.vault_needs_update_cause(), t.update_open_again_fix())
        }
        _ => (t.vault_not_read_cause(), t.nothing_lost_fix()),
    }
}
