# Pending Changes lie beside the log, and what this Device writes is signed only at Accept

Some changes must show before they are kept (#111): what an AI Task did (#112), a Shared Area offered from afar (#108), a stranger's Sent Item until "Show" (#91), a bundle with a lot in it. The user sees them as if they were stored, and Accept or Decline decides. Declining must leave nothing behind, and the user's own edits made in the meantime must not be lost. Both cores do it the same way (`pending.dart`, `pending.rs`).

## The pending file

Each pending set is one file next to the store, `pending/<id>.json`, written beside and renamed so a kill leaves the old one. Its keys are sorted and it has no spaces: `{"changes":[…],"id":…,"keys":[…],"kind":…,"made":…,"payload":…,"shown":…}`. `kind` says what the set is for (`ai-task`, `area-offer`, `sent-item`, `bundle`; the ticket that opens it names it), `made` is when it was opened, `keys` holds the key records of the Devices that signed its entries, and `payload` holds what the ticket that opened it applies itself at Accept, opaque to the core. A `shown` set is read through; any other set is read only by itself. As for every version marker (ADR 0019), no `v` is written, and a file with a `v` above 1 is left on disk unread. The changes are kept in the order they came, and each is one of two kinds:

- A draft, `{"date":…,"entity":…,"field":…,"made":…,"mark":…,"value":…}`, is a change this Device makes. It has no dseq and no signature. `date` is null unless the change carries a date of its own.
- A received entry, `{"entry":<its line as it came>,"mark":…}`, is an entry another Device wrote and signed.

`mark` is the log's high-water mark (`currentSeq`) when the change came in.

## Why two kinds, and why drafts are signed only at Accept

What this Device writes has no number and no signature until Accept. At Accept each draft is appended like any other edit: it is numbered after everything the Device wrote meanwhile, its causal context (ADR 0011) is the log's at that moment, and it is dated as it was drafted. If the dseq were taken when drafting, a Decline would leave a hole in this Device's own sequence, and the user's edits made meanwhile would be numbered after entries that never come. Every Device would hold a vector past entries it can never receive, as with a block's refusals (ADR 0021). A signature made when drafting would also be a statement that could leave the Device in a Backup or a copy of the store and be taken in later, though the user never accepted it. The AI Task is drafts only. So is a Sent Item: accepting one writes a new Note of the receiver's own (ADR 0015), and the file itself, with the sender's Profile, goes in `payload`.

Entries other Devices wrote are already numbered and signed by their authors and cannot be changed. They are kept as they came and taken in at Accept as any import is. An Area offered from afar is the Area's history, received, with the Area key and the Profiles in `payload`. A bundle is received entries, with its keys in `keys`.

A received entry is checked when it comes in, since a shown set shows it at once. Its signature must hold under a key pinned here or one of the set's keys, but nothing is pinned until Accept, so a Decline stores nothing, not a key either. An entry claiming this Device is refused. So is one a blocked person's Device wrote that an import would not keep (ADR 0021). Her Devices are judged by the grants of the log and of the set together, since an offered Area may be where a grant names her. Such a refusal is not noted as a refusal mark: those marks count what the log refused. A block also takes her entries out of every set already held.

## What sees a set, and what does not

The reads of what the store holds now see the shown sets on top of the log, oldest set first: a Note's or Routine's fields, the lists of entities and fields, the Conflicts. A draft shows as this Device's entry, numbered below zero so it is numbered nowhere. A received entry shows unless the log holds it by now. A change that one of the user's own edits made since overrides at Accept (below) does not show either, so the reads show what Accept will keep. What the roles leave out of the reads is judged with the shown sets too.

Nothing that decides trust or what leaves the Device sees a set:

- the roles of the Shared Areas as the log has them (`judged`), and with them whose Devices are blocked, what is refused and deleted, and the version vector;
- where an entity is sealed and which segment it goes into (`scopeOf`), so a pending filing moves nothing into a Shared Area;
- the fields only the user's own Devices count for, such as a block, an Override or a Sent Item's link (`currentByOwn`);
- the numbering of new entries, and everything held, relayed or backed up.

The one exception is the causal context of an edit made while a set is shown: what the set holds of other Devices counts as seen, since the user saw it when they wrote. Such an edit does not stand as a Conflict against the value it replaced, before Accept or after. The ticket that shows a set pauses the sync while the set waits, as the AI Task does (#112), so that Sent Items and syncs carry nothing that is undecided.

## Accept, Decline and the user's own edits

Accept first takes the received entries in through the import (ADR 0002, 0013, 0021), which pins their keys, refuses what it refuses and enforces the blocks. It then appends the drafts in order. Where one of the user's own Devices edited the same field after the change came in, the user's edit stays. "Own Devices" means this Device, one paired as its own, or one whose key is bound to the own Profile (ADR 0017). Edits another person's Device wrote do not count. A Routine's schedule counts as one field, as it conflicts as one. A draft over such an edit is not written. A received value over it is answered by writing the user's value again, where the received one would show or would stand as a Conflict with the user's edit, which can happen when the set was not shown. The written value is dated at Accept, and its causal context covers both, which ends the Conflict. Accept returns what it took in, what it wrote and the fields it kept for the review, each with the pending and the kept value. The mark `pending:<id>` (a local setting) is written in the same transaction as the drafts, so a restart before the file is gone deletes it instead of accepting twice.

Decline deletes the file, and the log never saw anything of the set. A set survives a restart: its file is read when the store opens, and the decision waits.

The phone's background sync opens a second store on the same files. So every call that changes a set, or asks for one, reads its file first and does nothing to a set whose file is gone or whose Accept is marked. The reads use the sets as that store last read them.

## Considered Options

- Pending entries in the log's own table, marked (rejected: every path that holds, relays, numbers or backs up entries would have to skip them, and one path that forgot would pass them on).
- Signing and numbering drafts when they are made (rejected: a Decline leaves holes in the Device's sequence, and signed statements the user never accepted).
- One file for all pending changes (rejected: an AI Task and an offered Area are decided apart).
- Holding the user's own edits back while a set waits (rejected: capturing and editing never wait).
- Letting the reads of trust see the shown sets (rejected: a set from someone else could make a grant, a block or a filing count before anyone accepted it).

## Consequences

- Backups carry no pending sets; a restored Device has none.
- The bytes of photos and recordings a received set refers to are not kept by the core. The ticket that keeps them with its set deletes them at Decline, or drops what the log does not refer to then (ADR 0020).
- An edit the user made on something only a declined set held, such as a Note the AI drafted, stays in the log without the rest of it and shows nowhere.
- An edit made while a set was shown counts that set's entries as seen. If the set is declined and the same entries come later by sync, they are older than the edit and bring no Conflict.
- Pending files are plain JSON next to the database, as unprotected as it is.

## Changes to earlier decisions

- ADR 0002: entries this Device writes are numbered and signed when they reach the log, so changes the user has not accepted yet wait beside it.
- ADR 0011: an edit's causal context counts the entries of the shown pending sets as seen.
- ADR 0021: a blocked person's entries never enter a pending set, and a block removes them from the sets already held.
