# Blocking a person refuses their entries, by the Device that wrote them, and deletes what is held

A user can block another person (#90): nothing that person wrote reaches the user's screen or storage, whichever Device relays it, and no Sent Item from them is shown, but their check-offs stay so the Agenda tells the truth. The person is not told. Both cores do it the same way (`blocks.dart`, `blocks.rs`).

## The block

The block is the entity `block:<profile id>` in the log, the id the person's Profile stands for through its aliases when it is written, with one field `blocked`: the name the person showed under then, so every one of the user's Devices lists them by it, also one that never kept their Profile; null unblocks. Like an Override (ADR 0017) it lies in no Area, so it is sealed under the personal key, goes with every sync between one's own Devices and never to another person's Device, a Shared Area's segments, a bundle for someone else or a Sent Item. Only an entry one of the user's own Devices wrote counts (this one, one paired as its own, or one whose key is bound to the own Profile), the newest such one: a block another person's modified app slips into the log blocks nobody. The own Profile cannot be blocked.

## Her Devices

A blocked person's Devices are those a grant in a Shared Area names her for (ADR 0013): the person of a Device's last grant that named anyone, through the aliases, in any Area held here. Never this Device nor one of the user's own, whatever a grant says, so a Manager's modified app cannot make the user block themselves. A Profile's binding does not count: any key already bound to a Profile can bind another (ADR 0017), so a person could otherwise claim somebody else's Device.

## What is kept, and in which shape

Of what her Devices write, only entries that carry no text, picture or sound and keep the Shared Areas whole are kept: a check-off or its undoing (`done@<occurrence>` on a Routine), a grant (`role:<device>` on an Area), and a Note's `$type` and filing (`place`). The last two say where the Routines and Sections other people hung on her Note lie, so they stay judged in their Area and sealed under its key; the Note itself has no capture and so shows nowhere, in no list, no Search and no count. A Routine spawned from it shows as one whose Note is gone. Each kept entry must have its exact shape, so nothing can hide in it: version 1 with no causal context and no key of a later version, entity ids and devices of lower-case letters, digits and dashes up to 64 characters, an Occurrence key written as the apps write it, a value that is an instant as both cores write it or null, a grant in its canonical form naming such an id, `$type` `note`, a filing naming an Area or Topic, and an author of at most 40 code points (the apps write none). Everything else of hers is refused: Notes' texts and captures, Sections, photos, recordings, Transcripts, Photo Texts, names, Routines, and every field a later version may add.

## Refused at import, deleted when held

Unlike every other judgement (ADR 0013), this one is made at import: an entry from her Device that is not kept is not stored, once its signature holds, so a forged entry moves nothing. A Device found to be hers only later, when a grant naming her arrives after the entries, is handled by the sweep that follows every sync bringing anything, the block and the unblock themselves: every entry of hers held that is not kept is deleted. This is the one place anything leaves the log (ADR 0002). The bytes of her photos and recordings that nothing else in the log refers to go with them; since the log no longer refers to them, they are neither asked for, nor taken from another Device, nor published. Refusals are not reported as forged entries.

Each refusal is noted per Device as a local setting `refused:<device>`, keys sorted and no spaces: `{"runs":[{"from":…,"scope":…,"to":…}]}`, the runs of dseqs refused, each with the scope its entities lay in, an Area or `personal` (ADR 0012). The scope is read where the entity lay: at import once the kept entries of the batch are in, a Note by its filing and a Section by the Note it names; at the block before anything is deleted. Each of the user's Devices refuses on its own, so the mark is not synced. While she is blocked, the version vector a Device tells others (ADR 0002) is raised to the newest dseq refused, so nobody sends her entries again. Unblocking releases the mark (`"released":true`): only runs in a Shared Area this Device still holds a role in are wanted, the vector stands just below the first wanted dseq that is not held, so the next sync with a Device that still holds her entries brings them back, and runs held again or no longer wanted are dropped, the mark with the last of them. Her dseqs run across everything she writes, her own Notes and Areas the user is not in among them; those never come and are never waited for. Causal contexts (ADR 0011) keep reading the plain vector of what is held.

## The shared folder

The app does not touch the shared folder: her segments and blobs stay in it, encrypted. The segments a blocked person's Devices write are never opened; what another Device relays of hers in its own segments is refused like any import. Unblocking makes the next folder sync read every segment whole again, so what was refused comes back from wherever it lies; the store skips what it holds.

## Her Profile and Sent Items

Her Profile stays kept, for the "Blocked" list and her Profile page, but its picture is taken out of the record (`"stripped":true` in `person:<id>`), so the record no longer matches its signature: it is never passed on, to one's own Devices or anyone else, and a newer record of hers is refused while she is blocked. After an unblock, any record of hers at the same version or newer replaces it. People leaves her out; the "Blocked" list shows her without a picture.

A Sent Item whose Profile id is blocked, or which a blocked person's Device signed, is refused before anything of it is shown, and cannot be accepted, updated or cancelled. "Decline and block" in a preview keeps her Profile without its picture and blocks her; for an id kept here under another first key, a copied id, it blocks nobody. Notes accepted from her before the block were written by the user's own Devices and stay.

## Managers and leaving an Area

A blocked Manager's Routines, Reminders, Area and Topic names would go half missing, so blocking a person who is a Manager of a Shared Area this Device holds a role in is only possible together with leaving it; without being asked to leave, blocking changes nothing. Leaving ends the grants of every Device of the user's in the Area, this Device's last. The roles change for it: a grant set to null by a Device counts when the grant it ends names the same person as the signer's own, so a Member can leave; a Member still ends nobody else's grant. What the Area held stays on the user's Devices as it was, and nothing more of it comes or goes.

## Considered Options

- Hiding her entries in the projection, as the roles do (rejected: what she wrote, possibly illegal material, would still be stored and relayed).
- Finding her Devices through her Profile's binding (rejected: a key bound to a Profile can bind anybody else's key).
- Deleting her Notes whole, `$type` and filing too (rejected: a Reminder a Manager made of her Note, and Sections others added to it, would fall out of their Area and stop syncing with it).
- A refusal record that syncs (rejected: each of the user's Devices holds and refuses its own entries; the block is what syncs).
- Only the lowest and the highest dseq refused (rejected: the dseqs in between include her own Notes and Areas the user is not in, which never come; the vector would wait for them forever, and every sync would send all of hers above it again).

## Consequences

- Backups made before the block are not touched; restoring one restores her content until the block arrives again with the next sync from another of the user's Devices.
- Her check-offs reach the user only through LAN syncs and other Devices' segments, since her own segments are never opened.
- A refused entry that no Device holds any more, or one of a Note she has since moved out of the Area, keeps her vector low after an unblock: LAN peers send her entries above it again at every sync, until it comes or she is blocked again.
- A Device that takes in the block without ever holding her content, one paired as the user's own after the block or restored from a Backup made after it, has no marks: after an unblock it cannot fetch again what lies below her newest check-off it holds.
- A blocked person made Manager later, of an Area the user stays in, is not caught: her Routines and names there go missing on the user's Devices.
- A blocked person who takes another Profile over (ADR 0017) is somebody new to the block. A Member can name a Device that never held a grant in an Area as her own person there (ADR 0013), which then counts as hers.
- Other people of a Shared Area keep everything; the blocked person sees nothing but, for a Manager, that the user left.

## Changes to earlier decisions

- ADR 0002: entries of a blocked person's Devices are deleted from the log, the only deletion there is; the version vector a Device tells counts what it refused.
- ADR 0012: segments of a blocked person's Devices are not read; an unblock reads the folder whole again.
- ADR 0013: a blocked person's entries are refused at import, not judged later; a person may end the grants of their own Devices, and so leave an Area.
- ADR 0015: a Sent Item from a blocked person is refused before its preview.
- ADR 0017: a blocked person's Profile is kept without its picture, not passed on, and not updated while blocked.
- ADR 0020: a blocked person's photos and recordings are deleted at the block, with no removal entry; their copies in the shared folder stay.
