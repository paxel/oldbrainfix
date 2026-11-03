# A Pack is a file of copies

Someone built a big set of Tallies, Sessions and other Notes, and a friend asks "can you send me these?" (#164, Q453–Q474). Not to do them together, which is a Shared Area, only so the friend does not have to type them all. A Pack is that: an unencrypted `.obfpack` file (`application/vnd.oldbrainfix.pack`) holding copies of Notes picked from an Area, a Topic or a Note, handed over through any messenger. The receiver keeps the copies as their own. Nothing links them back: no update, no cancellation, no sync. Both cores write and read the same bytes (`packs.dart`, `packs.rs`, the fixture `pack`).

A Pack is no Sent Item (one Note, encrypted when the receiver's keys are known, owned by the sender, who may update or cancel it, ADR 0009), no Share to an app (a Note handed to another app as text and files) and no Import (Notes made from another app's files, ADR 0036). The terms stay apart in the code too: the Pack reuses the Sent Item's way of writing Lines, Sections and Routines, and the Profile card's signature and blocked check (ADR 0017, #144), under names of its own.

## What a Note brings

A Note brings what it is (Q455): its text, each Box in its state now as a Sent Item carries it (ADR 0035), its Sections' texts with their formats, its kind with what makes it one (a Checklist's Lines unticked, a Shopping List's Lines open with their reappear times, a Tally's fields with their units, targets, rules, hidden flags and what each multiplies, a Session's pool, counts and rule), its Routines and Reminders as their schedules, and an open Frog (Q473). It never brings a Tally's entries, ticks of Lines, check-offs of Occurrences or Frogs, its Capture Context, its Importance or its history. A Section carries no date: the copies are captured when they are added, the Sections a microsecond apart after the Note in their order. A Section made by merging a Note brings its text, not that Note's photos or recordings.

- **Photos and recordings** (Q456) are items of their own, unticked at the start, each with its size. Only files on the Device can go.
- **Photo Texts and Transcripts** (Q457) are items of their own, ticked at the start, and go without their file too. A text without its file arrives as one whose file was removed (ADR 0025): `photo:<hash>` or `audio:<hash>` null, the text beside it, so it shows and the file can never be asked for.
- **Reminders** (Q465) are unticked at the start. A ticked one keeps its day and time; one already past when it is added, a day before or today at an earlier minute, by the receiver's wall clock, is left out, so the Note arrives without that due time.
- **Notes by other people** of a Shared Area (Q466) are offered as the user's own are.
- **Areas** (Q469) give only their names and the names of the Topics the Notes are filed into. A locked Area is unlocked before its menu offers anything, as for its other actions, and a Tally a Session picks from a locked Area that is closed now stays out, as the app shows it nowhere then (`packOf` takes the closed Areas).

## The tree

"Export as Pack…" sits in the menu of an Area and of a Topic, in a Note card's ⋮ and on the Note editor's bar (Q467). The tree shows only what it was started from (Q468): from a Note, that Note; from a Topic, its Notes; from an Area, its own Notes and then its Topics with theirs, newest capture first. Under each Note lie its photos, each with its Photo Text, its recordings, each with its Transcript, its Routines and Reminders and its Frog. A Session pulls in the Tallies of its pool that lie elsewhere; they show under the Session, keep their own Area and Topic, and land in a new Area of that name (Q461). One Pack Area stands for each source Area, so a Tally of another Topic of the same Area lands in the same new Area, in a Topic of its own.

The tree is the same on both sides (Q459): the core builds it from the Pack itself (`PackTree.tree`), the exporter's from the Pack of everything the tree may offer (`packOf`), the receiver's from the file. Its items are ticked by keys that name the Pack's Notes by their place in it (`n3`, `n3.p0`, `n3.pt0`, `n3.r1`, `n3.f`), never by the sender's ids. Everything starts ticked but Reminders and files on export, everything on import; "Tick all" and "Untick all" set them all.

**The Session rule** (Q458) holds on both sides: ticking a Session ticks the Tallies of its pool; unticking a Tally takes it out of the pool of every Session in the Pack, and a Session left with none of its pool ticked is unticked too. A Session ticked by hand with no Tally of its pool ticked still stays out: the core needs at least one Tally in a pool.

**The size** (Q470) has no limit and follows the ticks: each ticked Note counts the length of its own JSON, a ticked file its base64, a ticked text its UTF-8 bytes, a Routine its JSON; the items under a Note not ticked count nothing.

## The file

The file is one JSON object, keys sorted and no spaces: `{"pack":<payload>,"sig":<signature>,"type":"oldbrainfix-pack"}`. The payload is a string, so the signature covers exactly the bytes sent: `{"areas":[{"name":…,"topics":[…]}],"notes":[…],"profile":<card>}`. A Note says its Area and Topic by their index in `areas`, a Session its pool by the indices of its Tallies in `notes`, a multiplying field the index of the field it multiplies; a Tally pulled in from outside says `"outside":true`. Files go inline as standard base64 under `bytes`, beside their `hash` and their `text`; the bytes are what the sender's store keeps, so a photo arrives as it was kept, without its metadata.

A Pack is JSON as a Sent Item and a Profile card file are, so the phone asks for a Pack first and reads it whole: unlike them it has no size limit. Every other file is read only as far as its first byte when that opens no JSON object.

**Later formats** say `format`, which this version never writes. Only an absent `format` or the whole number 1 is this version's; `2`, `"1"`, `1.5` or `null` need an update (ADR 0034), and the app says so as for a Sent Item.

## Who it is from

**"Add my Profile"** (Q462), off at the start, puts the own Profile card into the payload, `<payload>.<signature>` as a `.obfcard` holds it, picture and all, and signs the whole payload with the Device that made the card, over `oldbrainfix:pack:<payload>`. Opening checks the card as a card file's is checked (its key record, its signature, its key bound to the Profile), then the Pack's signature with that key, before reading anything else: a Pack changed after it was signed, one with a card and no signature, or a signature and no card, is no Pack. One carrying the Profile of a blocked person, or made by a Device of theirs, is not opened (Q471). Without a Profile nothing can be checked and nothing is asked: the Pack is as anybody's file is.

With a Profile the page says "From <name>" with the picture, the user's Override applied when the person is kept here, and offers "Add <name> to People" while they are not, which opens the card as a `.obfcard` would. A Pack the user made shows as one without a Profile.

**"Claim as mine"** (Q463), ticked at the start beside "Add", is only asked with a Profile. Ticked, the Notes are simply the receiver's. Unticked, each Note gets the field `packFrom` = `{"id":<Profile id>,"name":<name>}`: the "From <name>" line of its card and a badge in the Note editor, removed from the card's ⋮ or the badge, which writes the field null. It is a field of the Note, so it syncs and travels with it like any other; it shows the name the Profile has here when the person is kept, else the name the Pack carried.

## Where it lands

"Add" (Q460, Q461) writes everything in one batch, by this Device, captured now: each Area of the Pack a new private Area of its name, not locked, not shared, with default settings, its Topics new Topics in it. A name an Area has already gets a suffix the app hands in, "Health (Pack)", then "Health (Pack 2)", "(Pack 3)" and on; the Areas made in the same Add count as taken. Notes of no Area land in a new Area the app names ("From a Pack"), by the same rule. Nothing lands in an existing or a Shared Area, so nothing is Unsorted and no Nudge comes.

- Every entity gets a new id, so a help Note (ADR 0042) arrives as an ordinary Note.
- Routines (Q464) start on the day of Add by the receiver's wall clock, their times and rules as they are, so "ends after 3 times" counts from that day. A Reminder keeps its day (above). They are written, not spawned, as a Sent Item's are, and alarms follow as for any new Routine.
- A Frog arrives open; a Checklist's Lines unticked; a Shopping List's open with their reappear times, one week when a later version's time is not known here; a Tally's fields with new ids, the multiplied ones after all of them; a Session's setup last, its pool naming the new Notes.
- Opening the same Pack twice adds the copies twice (Q453): they are copies only.

## The AI port

A Pack has no AI port tool to make or open one (ADR 0026): like an Import (ADR 0036) it is a person's own act with the system's share sheet and a file they opened, not something to set up, and the Notes it makes are read and changed through the tools Notes already have. The one new field, `packFrom`, is read there with the Note it is on and is never set or removed by the AI port.

## The desktop

The desktop core reads and writes the same bytes now (`packs.rs`). The desktop's screens follow in their own ticket (#165); until then the desktop makes and opens no Pack.

## Considered

- A zip with the files beside a JSON (rejected: the two cores' zip writers lay out their headers apart, as for bundles, so the file could not be the same bytes; base64 inline is a third larger).
- Encrypting the Pack (rejected, Q453: it is a file of copies for anyone the sender hands it to; a Sent Item is the encrypted way).
- The payload as a JSON object signed over its canonical form (rejected: both cores would have to re-encode a parsed object byte for byte; a string is signed as it lies in the file, as a Sent Item's item is).
- Keeping the sender's ids (rejected: a help Note's fixed id, or a Note the receiver had from a Shared Area, would mix with the copy).
- Landing in an Area of the same name (rejected, Q460: the copies could no longer be told from the receiver's own, nor deleted with their Area).
- One new Area for each Tally pulled in from another Topic of the same Area (rejected: the Tally belongs to that Area; a second "Health (Pack 2)" beside "Health (Pack)" would split what was one).
- Dating the copies and their Sections as the sender's (rejected, Q455: the dates are Capture Context).
- Bringing a merged Note's files with its Section (rejected for now: the tree offers the Note's own files; the merged text arrives).

## Consequences

- The `pack` fixture holds a Health with every kind, a Session picking a Tally of another Area, Routines ending after three times, a Reminder to come and one past, an open and an eaten Frog, a photo and a recording with their texts; the tree with its keys, kinds, sizes and ticks; a Pack as the tree starts with both files, one of everything signed with Ada's Profile, each added by Ben, claimed and not, beside a Health of his own; and a changed Pack, one of a later format and one from a person blocked, all refused. Both cores write the same files and the same logs, byte for byte.
- A Pack can be large: photos travel in it base64. The tree says the size before anything is written.
