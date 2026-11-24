//! "Empty the trash now" (obf-1095), as the Dart core's `Trash`: what
//! the phone's 30-day purge removes, all at once, with the bytes of the
//! files of those Notes let go on this Device.

use std::collections::BTreeSet;
use std::time::Instant;

use crate::Result;
use crate::notes::{Notes, keys as note_keys};
use crate::places::{MoveProgress, Stopping};
use crate::routines::{Routines, keys as routine_keys};
use crate::store::Store;
use crate::sync::{BLOB_KINDS, split_blob_ref};

/// What "Empty the trash now" removes, or removed, as the Dart core's
/// `TrashEmptied`: the Notes and the Routines and Reminders by id, in
/// the order they are purged, and the files of those Notes whose bytes
/// left this Device.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrashEmptied {
    /// The Notes, merged ones not counted: they go with their target.
    pub notes: Vec<String>,
    /// The Routines and Reminders, those trashed with their Note
    /// included.
    pub routines: Vec<String>,
    /// The photos, recordings and attached files whose bytes were let
    /// go, as `photo/<hash>`, `audio/<hash>` and `file/<hash>`, sorted;
    /// none for [`Trash::to_empty`].
    pub blobs: Vec<String>,
}

impl TrashEmptied {
    /// Whether nothing is, or was, removed.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.routines.is_empty()
    }
}

/// How many Notes, or Routines and Reminders, [`Trash::empty`] purges in
/// one transaction, as the Dart core's `Trash.perBatch`: another
/// connection to the database, such as the window's while it runs beside
/// it, waits at most for so many.
pub const PER_BATCH: usize = 25;

/// The trash emptied at once (obf-1095).
pub struct Trash<'a> {
    store: &'a Store,
}

impl<'a> Trash<'a> {
    pub fn new(store: &'a Store) -> Trash<'a> {
        Trash { store }
    }

    /// What [`Trash::empty`] removes now, for the question asked before
    /// it: every Note and every Routine or Reminder in the trash that this
    /// Device may change, as the Dart core's 30-day purge removes them,
    /// however long they have been there. Those it may not change in a
    /// Shared Area stay, and are not counted.
    pub fn to_empty(&self) -> Result<TrashEmptied> {
        Ok(TrashEmptied {
            notes: self.notes()?,
            routines: self.routines()?,
            blobs: Vec::new(),
        })
    }

    /// "Empty the trash now": purges what [`Trash::to_empty`] names, as
    /// the Dart core's 30-day purge does, [`PER_BATCH`] at a time, each
    /// batch a transaction of its own, resting as long as it took before
    /// the next, as the desktop's Import does beside its window; then lets
    /// go of the bytes of the photos, recordings and files of those Notes,
    /// their Sections and the Notes merged into them, on this Device,
    /// unless a Note not purged still shows them. One brought back from
    /// the trash or purged meanwhile is left as it is. The log keeps every
    /// entry, so sync is unchanged; a file only a purged Note holds is no
    /// longer wanted from another Device. It cannot be undone. Frogs stay,
    /// as the 30-day purge leaves them (ADR 0027).
    pub fn empty(&self) -> Result<TrashEmptied> {
        self.empty_stopping(&|_, _| (), &|| false)
    }

    /// As [`Trash::empty`]: `progress` hears how many of the Notes it has
    /// gone through, before the first batch and after each; `stop` is
    /// asked before each batch. Once it says so, or a batch fails, what
    /// was purged stays purged and its bytes go all the same.
    pub fn empty_stopping(
        &self,
        progress: MoveProgress<'_>,
        stop: Stopping<'_>,
    ) -> Result<TrashEmptied> {
        let mut refs = BTreeSet::new();
        let mut emptied = TrashEmptied::default();
        let purged = self.purge(&mut refs, &mut emptied, progress, stop);
        if let Err(e) = purged {
            // What went wrong is told; letting go of the bytes of what
            // went before is tried all the same.
            let _ = self.drop_blobs(&refs);
            return Err(e);
        }
        emptied.blobs = self.drop_blobs(&refs)?;
        Ok(emptied)
    }

    /// The purge behind [`Trash::empty_stopping`]: adds the Notes and the
    /// Routines and Reminders it purged to `emptied`, and the files they
    /// held to `refs`, each once its batch is in.
    fn purge(
        &self,
        refs: &mut BTreeSet<String>,
        emptied: &mut TrashEmptied,
        progress: MoveProgress<'_>,
        stop: Stopping<'_>,
    ) -> Result<()> {
        let notes = Notes::new(self.store);
        let mut since: Option<Instant> = None;
        // Rests as long as the batch before took, so the window's reads
        // and writes, which wait for its transaction, get the database
        // between two batches.
        let mut next = || {
            if let Some(since) = since {
                std::thread::sleep(since.elapsed());
            }
            if stop() {
                return false;
            }
            since = Some(Instant::now());
            true
        };
        let selected = self.notes()?;
        // Read for all of them at once, before anything is written: one
        // pass, not one per Note.
        let mut sections = notes.sections_of(&selected)?;
        progress(0, selected.len());
        let mut at = 0;
        for ids in selected.chunks(PER_BATCH) {
            if !next() {
                return Ok(());
            }
            let (gone, held) = self.store.batch(|| {
                let mut gone = Vec::new();
                let mut purged = Vec::new();
                for id in ids {
                    if !self.still_purgeable(
                        id,
                        (note_keys::TYPE, note_keys::TYPE_NOTE),
                        note_keys::TRASHED,
                        &|id| self.store.may_change(id),
                    )? {
                        continue;
                    }
                    // A merged Note's Section is the Note itself; both
                    // keep their type under the same key.
                    for section in sections.remove(id).unwrap_or_default() {
                        if self.store.may_change(&section.id)? {
                            self.store
                                .append(&section.id, note_keys::TYPE, None, None)?;
                            purged.push(section.id);
                        }
                    }
                    self.store.append(id, note_keys::TYPE, None, None)?;
                    purged.push(id.clone());
                    gone.push(id.clone());
                }
                Ok((gone, self.refs_of(&purged)?))
            })?;
            emptied.notes.extend(gone);
            refs.extend(held);
            at += ids.len();
            progress(at, selected.len());
        }
        let selected = self.routines()?;
        for ids in selected.chunks(PER_BATCH) {
            if !next() {
                return Ok(());
            }
            let gone = self.store.batch(|| {
                let mut gone = Vec::new();
                for id in ids {
                    if self.still_purgeable(
                        id,
                        (routine_keys::TYPE, routine_keys::TYPE_ROUTINE),
                        routine_keys::TRASHED,
                        &|id| self.store.may_change_routine(id),
                    )? {
                        self.store.append(id, routine_keys::TYPE, None, None)?;
                        gone.push(id.clone());
                    }
                }
                Ok(gone)
            })?;
            emptied.routines.extend(gone);
        }
        Ok(())
    }

    /// Whether `id`, picked before an earlier batch, still has `type_` and
    /// is in the trash under `trashed`, also in the log, and this Device
    /// may still change it, as `may_change` tells: brought back or purged
    /// meanwhile, it stays as it is.
    fn still_purgeable(
        &self,
        id: &str,
        type_: (&str, &str),
        trashed: &str,
        may_change: &dyn Fn(&str) -> Result<bool>,
    ) -> Result<bool> {
        let fields = self.store.current_fields(id)?;
        Ok(fields.get(type_.0) == Some(type_.1)
            && fields.get(trashed).is_some()
            && self.store.logged(id, trashed)?.is_some()
            && may_change(id)?)
    }

    /// The photos, recordings and files the `purged` entities name, as
    /// `<kind>/<hash>`.
    fn refs_of(&self, purged: &[String]) -> Result<BTreeSet<String>> {
        let mut refs = BTreeSet::new();
        let held = self
            .store
            .current_fields_of(purged.iter().map(String::as_str))?;
        for fields in held.values() {
            for (field, _) in fields.iter() {
                for (kind, prefix) in BLOB_KINDS {
                    if let Some(hash) = field.strip_prefix(prefix) {
                        refs.insert(format!("{kind}/{hash}"));
                    }
                }
            }
        }
        Ok(refs)
    }

    /// Lets go of the bytes of `refs` on this Device, those a Note not
    /// purged still shows kept; returns those let go, sorted. Once the
    /// purge is in: the bytes are not part of a transaction.
    fn drop_blobs(&self, refs: &BTreeSet<String>) -> Result<Vec<String>> {
        let mut blobs = Vec::new();
        if refs.is_empty() {
            return Ok(blobs);
        }
        let known = self.store.known_blobs()?;
        for reference in refs {
            if known.contains(reference) {
                continue;
            }
            let (kind, hash) = split_blob_ref(reference);
            if self.store.drop_blob(kind, hash)? {
                blobs.push(reference.clone());
            }
        }
        Ok(blobs)
    }

    /// The Notes in the trash a purge removes, by id, as the Dart core's
    /// `Notes.purgeable`: a merged Note goes with its target, never on
    /// its own; it is in the trash in the log, not only in a change still
    /// to be accepted or rejected (ADR 0022), which rejected brings it
    /// back; and what this Device may not change in a Shared Area waits
    /// for someone who may: written here, it would be ignored every time.
    fn notes(&self) -> Result<Vec<String>> {
        let notes = Notes::new(self.store);
        let mut ids = Vec::new();
        for id in self.store.entities_of_type(note_keys::TYPE_NOTE)? {
            if let Some(note) = notes.by_id(&id)?
                && !note.is_merged()
                && note.trashed.is_some()
                && self.store.logged(&id, note_keys::TRASHED)?.is_some()
                && self.store.may_change(&id)?
            {
                ids.push(id);
            }
        }
        Ok(ids)
    }

    /// The Routines and Reminders in the trash a purge removes, by id:
    /// in the trash in the log, as a Note, and only where this Device may
    /// change them (ADR 0013).
    fn routines(&self) -> Result<Vec<String>> {
        let routines = Routines::new(self.store);
        let mut ids = Vec::new();
        for id in self.store.entities_of_type(routine_keys::TYPE_ROUTINE)? {
            if let Some(routine) = routines.by_id(&id)?
                && routine.trashed.is_some()
                && self.store.logged(&id, routine_keys::TRASHED)?.is_some()
                && self.store.may_change_routine(&id)?
            {
                ids.push(id);
            }
        }
        Ok(ids)
    }
}

#[cfg(test)]
mod tests;
