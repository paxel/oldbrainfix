//! "Empty the trash now" (obf-1095), as the Dart core's `Trash`: what
//! the phone's 30-day purge removes, all at once, with the bytes of the
//! files of those Notes let go on this Device.

use std::collections::BTreeSet;

use crate::Result;
use crate::notes::{Notes, keys as note_keys};
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

    /// "Empty the trash now": purges what [`Trash::to_empty`] names in one
    /// batch, as the Dart core's 30-day purge does, then lets go of the
    /// bytes of the photos, recordings and files of those Notes, their
    /// Sections and the Notes merged into them, on this Device, unless a
    /// Note not purged still shows them. The log keeps every entry, so
    /// sync is unchanged; a file only a purged Note holds is no longer
    /// wanted from another Device. It cannot be undone. Frogs stay, as the
    /// 30-day purge leaves them (ADR 0027).
    pub fn empty(&self) -> Result<TrashEmptied> {
        let notes = Notes::new(self.store);
        let mut refs = BTreeSet::new();
        let (gone, gone_routines) = self.store.batch(|| {
            let gone = self.notes()?;
            // Read for all of them at once, before anything is written:
            // one pass, not one per Note.
            let mut sections = notes.sections_of(&gone)?;
            let mut purged = Vec::new();
            for id in &gone {
                // A merged Note's Section is the Note itself; both keep
                // their type under the same key.
                for section in sections.remove(id).unwrap_or_default() {
                    if self.store.may_change(&section.id)? {
                        self.store
                            .append(&section.id, note_keys::TYPE, None, None)?;
                        purged.push(section.id);
                    }
                }
                self.store.append(id, note_keys::TYPE, None, None)?;
                purged.push(id.clone());
            }
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
            let gone_routines = self.routines()?;
            for id in &gone_routines {
                self.store.append(id, routine_keys::TYPE, None, None)?;
            }
            Ok((gone, gone_routines))
        })?;
        // Once the purge is in: the bytes are not part of the transaction.
        let known = self.store.known_blobs()?;
        let mut blobs = Vec::new();
        for reference in refs {
            if known.contains(&reference) {
                continue;
            }
            let (kind, hash) = split_blob_ref(&reference);
            if self.store.drop_blob(kind, hash)? {
                blobs.push(reference);
            }
        }
        Ok(TrashEmptied {
            notes: gone,
            routines: gone_routines,
            blobs,
        })
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
