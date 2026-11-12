//! What Ctrl+Z and Edit ▸ Undo take back on the desktop (#268, Q646
//! (c)): the last delete of what can be restored, check-off, filing or
//! Merge, in place of asking before it. Deleting what comes back asks
//! nothing; deleting for good still asks once.
//!
//! An undo uses the restore the cores have and writes nothing more than
//! it does. Where a sync, the Trash or another step got there first and
//! nothing is left to bring back, it does nothing.

use oldbrainfix_core::{
    Checklists, Frogs, Notes, Occurrence, Places, Routines, ShoppingLists, Store, Tallies,
};

/// The one thing the next undo takes back.
#[derive(Debug, Clone, PartialEq)]
pub enum Undo {
    /// A Note moved to the trash, with its Routines and Reminders.
    Note(String),
    /// A Routine or Reminder moved to the trash on its own.
    Routine(String),
    /// A Frog moved to the trash.
    Frog(String),
    /// A Tally entry removed.
    Entry(String),
    /// The Line `id` of the Checklist, or with `shopping` the Shopping
    /// List, of the Note `note`, deleted.
    Line {
        note: String,
        id: String,
        shopping: bool,
    },
    /// The "From …" badge of the Note removed.
    PackFrom(String),
    /// The Photo Text (`photo`) or Transcript of `hash` of the Note
    /// `note` removed, with the text it had.
    Text {
        note: String,
        hash: String,
        photo: bool,
        text: String,
    },
    /// The Note with this id merged into another.
    Merge(String),
    /// The Occurrence of the Routine checked off, and the Note that went
    /// to the trash with it, if one did (#211, #215).
    Done {
        routine: String,
        occurrence: Occurrence,
        trashed: Option<String>,
    },
    /// The Frog checked off.
    FrogEaten(String),
    /// Notes filed, each with the Area or Topic it was in before, or
    /// none when it was filed nowhere.
    Filed(Vec<(String, Option<String>)>),
}

impl Undo {
    /// Takes it back as far as it still can be.
    pub fn apply(self, store: &Store) -> oldbrainfix_core::Result<()> {
        match self {
            Undo::Note(id) => restore_note(store, &id)?,
            Undo::Routine(id) => {
                let routines = Routines::new(store);
                if routines.by_id(&id)?.is_some_and(|r| r.is_trashed())
                    && store.may_change_routine(&id)?
                {
                    routines.restore(&id)?;
                }
            }
            Undo::Frog(id) => {
                let frogs = Frogs::new(store);
                if frogs.by_id(&id)?.is_some_and(|f| f.is_trashed())
                    && store.may_manage(store.area_of(&id)?.as_deref())?
                {
                    frogs.restore(&id)?;
                }
            }
            Undo::Entry(id) => {
                let tallies = Tallies::new(store);
                if tallies.entry_by_id(&id)?.is_some_and(|e| e.is_trashed())
                    && store.may_change(&id)?
                {
                    tallies.restore_entry(&id)?;
                }
            }
            Undo::Line { note, id, shopping } => restore_line(store, &note, &id, shopping)?,
            Undo::PackFrom(note) => {
                if store.pack_from(&note)?.is_none() && store.may_change(&note)? {
                    store.restore_pack_from(&note)?;
                }
            }
            Undo::Text {
                note,
                hash,
                photo,
                text,
            } => {
                let notes = Notes::new(store);
                // Made again meanwhile, or the file gone: nothing to
                // bring back.
                let removed = notes.by_id(&note)?.is_some_and(|n| {
                    let (files, removed) = if photo {
                        (&n.photos, &n.removed_photo_texts)
                    } else {
                        (&n.recordings, &n.removed_transcripts)
                    };
                    files.contains(&hash) && removed.contains(&hash)
                });
                if removed && store.may_change(&note)? {
                    if photo {
                        notes.set_photo_text(&note, &hash, &text)?;
                    } else {
                        notes.set_transcript(&note, &hash, &text)?;
                    }
                }
            }
            Undo::Merge(id) => {
                let notes = Notes::new(store);
                if notes.may_undo_merge(&id)? {
                    notes.undo_merge(&id)?;
                }
            }
            Undo::Done {
                routine,
                occurrence,
                trashed,
            } => {
                let routines = Routines::new(store);
                let done = routines
                    .by_id(&routine)?
                    .is_some_and(|r| r.done_at(&occurrence).is_some());
                if done && store.may_check_off(&routine)? {
                    // The Note first: a Reminder in the trash takes no
                    // uncheck, and it came back with its Note.
                    store.batch(|| {
                        if let Some(note) = &trashed {
                            restore_note(store, note)?;
                        }
                        let back = routines.by_id(&routine)?.is_some_and(|r| !r.is_trashed());
                        if back {
                            routines.unmark_done(&routine, &occurrence)?;
                        }
                        Ok(())
                    })?;
                }
            }
            Undo::FrogEaten(id) => {
                let frogs = Frogs::new(store);
                if frogs.by_id(&id)?.is_some_and(|f| f.is_done()) && store.may_check_off(&id)? {
                    frogs.unmark_done(&id)?;
                }
            }
            Undo::Filed(moves) => {
                let notes = Notes::new(store);
                let places = Places::new(store);
                store.batch(|| {
                    for (note, place) in &moves {
                        let listed = notes.by_id(note)?.is_some_and(|n| !n.is_trashed());
                        if !listed {
                            continue;
                        }
                        match place {
                            Some(place) => {
                                if store.may_change(note)?
                                    && places.is_place(place)?
                                    && store.may_write_in(place)?
                                {
                                    notes.file(note, place)?;
                                }
                            }
                            // Back to Unsorted, where it came from.
                            None => {
                                if notes.may_unfile(note)? {
                                    notes.unfile(note)?;
                                }
                            }
                        }
                    }
                    Ok(())
                })?;
            }
        }
        Ok(())
    }
}

/// Brings the Note `id` back from the trash with its Routines and
/// Reminders, while it is there and this Device may change it.
fn restore_note(store: &Store, id: &str) -> oldbrainfix_core::Result<()> {
    let trashed = Notes::new(store).by_id(id)?.is_some_and(|n| n.is_trashed());
    if trashed && store.may_change(id)? {
        Routines::new(store).restore_note(id)?;
    }
    Ok(())
}

/// Brings the deleted Line `id` of the list of `note` back, while its
/// "Removed Lines" lists it and this Device may change it.
fn restore_line(
    store: &Store,
    note: &str,
    id: &str,
    shopping: bool,
) -> oldbrainfix_core::Result<()> {
    if shopping {
        let lists = ShoppingLists::new(store);
        if lists.deleted_lines(note)?.iter().any(|l| l.id == id) && lists.may_edit(id)? {
            lists.restore(id)?;
        }
    } else {
        let lists = Checklists::new(store);
        if lists.deleted_lines(note)?.iter().any(|l| l.id == id) && lists.may_edit(id)? {
            lists.restore(id)?;
        }
    }
    Ok(())
}
