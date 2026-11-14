# The vault is a folder of Markdown written by hand and rebuilt only into an empty app

A second brain must not lock its owner in (#267, Q615, Q617, Q626–Q630, Q633–Q635). The vault is a way out of Old Brain Fix for the day the user leaves for another tool, Obsidian or a git host, and a way back if they return. It is not a second place to work in: the signed log stays the only store (ADR 0002), and nothing written in the vault flows back on its own. Only the desktop writes and reads it (Q634), since a vault is a folder of thousands of files; the Rust core holds both directions (`vault.rs`), and there is no Dart twin and no fixture.

## Export, by hand, into a new folder

File ▸ "Export as vault…" asks for a folder and writes the vault into a new folder inside it, `oldbrainfix-vault-<date>`, with `-2`, `-3` and so on after it when that name is taken. Each export is written anew and complete; nothing already in the chosen folder is read, changed or deleted, so an older vault or the user's own files beside it stay as they are. When writing fails, the new folder is removed again, and only it. A folder kept current by the app was rejected (Q633): it invites editing in another tool, and those edits never come back.

## Layout

- A folder per Area, by the name it shows in the app's language; a subfolder per Topic; the Notes filed nowhere in a folder named as Unsorted shows. Areas and Topics are ordered by that name, then by id, so the same log gives the same folders.
- Each Note is one Markdown file named by its first line with a letter or digit, as it reads, and its capture date on the wall calendar, `Dentist 2026-11-03.md`; a Note without words by its file's name, else as "Note". Merged Notes are no files of their own: they are Sections of the Note they were merged into.
- Its photos, voice recordings and attached files lie in a folder `attachments` beside the file, a photo as `photo-<12 hex of its hash>.<jpg|png|gif|webp>`, a recording as `recording-<12 hex>.<wav|ogg|mp3|m4a>`, as "Save as Markdown…" names them (#129), an attached file under its own name. A file several Notes of a folder hold is written once. Only blobs this Device holds go along, one at a time in memory; the others are named in the properties only and counted.
- Names are made safe for every file system: no `/ \ : * ? " < > |`, none of `# ^ [ ]` that break a link in Obsidian, no control character or mark that turns the direction of text, blanks as one space, no dot or space at either end (no hidden file, nothing Windows refuses), at most 60 characters and 180 bytes before the date, a name Windows keeps for a device (`CON`, `COM1`) with `_` after it. Names are unique in their folder compared in lower case, as Windows and macOS compare them, the later ones with " (2)", " (3)" and so on; no Area, Topic or file is named `attachments`.

## A Note's file

The file is YAML front matter, then the Markdown:

```
---
oldbrainfix: 1
id: "note:…"
captured: "2026-11-03T08:00:00.000Z"
format: "plain"
area: "Health"
…
---
The Note's text, byte for byte

<!-- oldbrainfix:section section:… -->
*2026-11-04 10:30*

A Section's text, byte for byte

<!-- oldbrainfix:shown -->

- [ ] what the app draws for the other tool
```

- **Properties**: each one line, `key: <JSON value>`, JSON being a flow value every YAML reader takes; the few characters YAML does not allow unescaped (delete, the C1 controls, the byte order mark, the line and paragraph separators) are written as `\u` escapes. `oldbrainfix` holds the vault's format, 1, and marks the file as Old Brain Fix's (ADR 0034). Then what a rebuild needs and the Markdown does not show: `id`, `captured`, `format` (and `textVersion` for a text of another structure), the Area by `area`, `areaId`, its Starter Set slug, colour, icon and Hidden from AI mark, the Topic by `topic`, `topicId` and its slug, `importance` when not normal, `author`, the Note's own `hiddenFromAi` mark, `spawned`, the `kind` with its `lines` (id, text, under the Line above, when checked, a Shopping List's reappear time), `tally` (bar, sets, fields with id, name, unit, rule, target, hidden, Check, multiplier; every entry with id, moment, values per set as the log keeps numbers, line and when removed) or `session` (pool by Note id, count, counts per weekday, rule), `routines` and `reminders` (id, start, times and RRULE as the log keeps them, the version of their structure, when last scheduled, own Re-nag and escalation, and by Occurrence their check-offs, skips, Line checks and a Session's picks and entries), `frogs` (when done), `agenda` (the user's own "Show on the Agenda" choice of a Checklist where it is not the default, and of a Shopping List as it shows, since a Device's default may differ; ADR 0030), `attachments` (kind, path in the vault or none, hash, text, a file's name, type and pages, whether it was removed while its text stays) and `sections` (id, capture time, format, whether a merged Note, its attachments).
- **Texts are written byte for byte** (Q626): they are Markdown already (ADR 0033), so nothing is converted, and what a conversion loses would be lost from the Note. A plain text is written as it is too; its `format` says so. Each box shows the glyph of its state, without its id (ADR 0035). Only one thing changes: a link to a Note (ADR 0046) in a formatted text becomes a relative link to that Note's file, its names %-encoded, `[Dentist](../Health/Dentist%202026-11-03.md)`, which Obsidian and git hosts follow; a link to a Note not in the vault, and every other link, stays as written. Nested tags stay as written (ADR 0032).
- **Sections** follow the text, oldest first, each after a marker line naming its id, an HTML comment other tools do not show, then its date in italics and a blank line.
- **What is drawn** for the other tool follows a marker of its own and is never read back: a Checklist's or Shopping List's Lines as a task list, a Tally's entries as a table (moment, each field shown, line), a Session's pool as links to its Tallies' files, and the Note's and its merged Notes' files as images and links. Every line of it starts with `- `, `|`, `[` or `!`, so it never holds a marker line.
- The text ends with one line break more than it has.

## What goes in, and what not

Everything this Device reads (Q627): other people's Notes in Shared Areas with their author, and Notes Hidden from AI, as the mark concerns the AI, not the user (ADR 0049). Left out are the Notes in the trash, which go anyway; the help Notes (ADR 0042), which the app makes again; Routines, Frogs and Tally entries in the trash but for removed entries, which can be restored; and the Capture Context, as the vault is not encrypted and the places, calendar events and Bluetooth devices of a capture are no business of a folder that may end up on a git host. Empty Areas and Topics hold nothing to keep and are not in it. Removed Lines, the person's settings (ADR 0040: the order of the Areas, Simple or Advanced, the Trash setting) and this Device's settings are not in it either; a rebuild starts with the defaults, as after a restore the Device's settings do (ADR 0016). While the AI port is open or an AI Task waits for its review, "Export as vault…" is greyed: the reads show the Task's drafts (ADR 0022), which are no Notes until accepted. An attached file's text, a Photo Text and a Transcript are properties, so a removed file's text stays.

## Rebuild, only into an empty Old Brain Fix

"Rebuild from a vault…" sits beside "Restore a Backup" on the first start and goes, as a restore does (ADR 0016), into an empty Old Brain Fix only: a log holding anything is refused before anything is read and again in the transaction that writes, and nothing changes. Working in two tools and then merging is not a case worth the conflicts it brings (Q629).

The folder and its folders are read for Markdown (`.md`, `.markdown`) and text (`.txt`) files, without hidden files and folders (`.obsidian`, `.git`), without the `attachments` folders and without following links. A file is Old Brain Fix's when its front matter holds `oldbrainfix`: of a later format the whole rebuild is refused as needing an update, and nothing is written; of this format it comes back with its properties when all of them can be read and `id`, `captured` and `format` are there. A file whose properties another tool rewrote so that one cannot be read is read as made elsewhere: its whole text, the front matter included, becomes the Note, so nothing of it is lost, but its Routines and kind do not come back. A file whose line breaks another tool turned into `\r\n`, as git does on Windows, is read with `\n` throughout, its texts included. A folder holding no Note is refused as no vault.

- **New ids** (Q630): every Note, Section, Line, Tally field and entry, Routine, Frog, Area and Topic gets a new id, every Area a new key, and every id a property, a field or a text names is written by its new one; a link to a Note's file of the vault becomes an app link to the new Note. The user's other Devices still holding the old log have nothing in common with it, and get the rebuilt Old Brain Fix only once emptied and paired again; kept ids would let an old phone merge the old and the new by latest wins. Two files holding the same id (a copy) are both rebuilt, the first one keeping the id for links.
- **Signed by this Device**: every entry is written by this Device. Edit history, signatures, box ticks, other people's authorship and their entries' writers do not come back; Shared Areas come back as the user's own, private. The `author` property is for the other tool only. Hidden from AI marks come back on Notes and Areas, so a hidden Note stays hidden from the AI.
- **Dates**: each entry is dated by the moment it says, a Note's by its capture, a schedule's by when it was last scheduled, so Occurrences before then are not Missed, a check-off by when it was checked off, never later than now, as an Import dates its Notes (ADR 0036).
- **Files** are read only from inside the vault: a path out of it, absolute, with `..`, `\`, `:` or a control character, or one whose link leads outside, is refused and counted as missing; larger than 256 MiB as well (ADR 0047). Each file is kept before the log names it, one at a time, under the hash of its bytes now, so a file changed in the vault is kept as it is. A removed one, or one not found, comes back as removed with its text, when it has one.
- **Files made elsewhere** become Notes as an Import makes them (ADR 0036): UTF-8 or Windows-1252, the file's name as the first line unless the text begins with it, its front matter kept as characters, the tags of its front matter (`tags:` as a list in brackets, lines under it, or words) as tags on a last line, each level of a nested tag as `text_tags::from_label` reads a label, those already in the text left out, dated when the file was last changed, at most 1 MiB and 256 KiB of text. They are filed by their folders: the first folder is the Area and the second the Topic, an Area or Topic of that name that the rebuild made taken, else a new one; a file at the top lands in an Area named after the vault's folder, as an Import names its Area. Their relative links to other files of the vault become app links. Obsidian's `[[wikilinks]]` and embeds stay as written, and files other than Markdown and text are not attached.
- The files are kept first, then the whole log is written in one transaction, so a rebuild that fails leaves the log empty and can be tried again.

## The first day back

A rebuilt Old Brain Fix is the old one on the day of the rebuild. Unsorted Notes come back Unsorted, so the one Nudge for them (CONTEXT.md) comes at once, as it would have on the old Device; the Missed Summary shows what was Missed in its last days, since the old schedules run on from when they were last scheduled; a list the user took off today's Agenda stays off. Nothing of this is softened: a Note Nudged or an Occurrence Missed is what the user would have seen had they stayed.

## No AI port tool, no phone

A vault is a person's own act with the system's folder picker, not something to set up, so it has no AI port tool (ADR 0026), as an Import has none (ADR 0036). The phone neither writes nor reads one.

## Considered Options

- A folder kept current, or a vault synced both ways (rejected, Q615, Q633): an outside edit would have to be signed as if a Device had written it, and the vault would invite work that never comes back.
- AsciiDoc (rejected, Q626): every text would be converted out and back, and relative links are standard Markdown.
- Plain text escaped as Markdown in the vault (rejected): the vault would hold another text than the Note, and the way back would have to unescape it.
- Properties as a YAML structure written and read by a YAML library (rejected): a library more, with its licences, for what one line of JSON per property does; a file whose properties another tool rewrote is read as made elsewhere instead.
- Reading the Markdown back for what it shows, Lines from a task list, entries from a table (rejected): their check times, ids and units are not in it; the properties are the truth, the drawing is for the other tool.
- Kept ids on the way back (rejected, Q630): see above.
- A rebuild into an Old Brain Fix holding Notes (rejected, Q629).
- The Capture Context in the properties (rejected): a plain folder is no place for where a person was.
- Writing over an older vault in the chosen folder (rejected): what the user put there would be lost.

## Consequences

- `Error::Vault` names why a rebuild was refused: not empty, no vault, a later format.
- The first start offers four choices; "Rebuild from a vault…" runs off the window's thread and is announced as a restore is.
- A rebuilt Old Brain Fix and the old one never sync with each other.
