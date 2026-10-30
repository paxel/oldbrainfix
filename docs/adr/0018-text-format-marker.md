# Every user text is stored with its format, and a format not known is never overwritten

Formatted text is planned for a later version (#57). So that a Device of this version cannot destroy formatted text written by a later one, this version stores the format of every text it writes, and leaves alone any text in a format it does not know (#58).

## The marker

The text of a Note and of a Section is stored as one value: `{"format":…,"text":…,"v":…}`, keys sorted and no spaces, the same bytes from both cores (`text_format.dart`, `text_format.rs`). `v` is the version of the value's structure: which keys it holds and what they mean. Version 1 holds `format` and `text`. `format` says how `text` is meant. Version 1 is the default, and a default is never written, so a missing `v` means 1 and this version writes `{"format":"plain","text":…}`: `"format":"plain"`, the text as typed, and no `v`. A later version that writes a higher `v` writes it out. Text and format sit in one entry, so they are signed, synced, judged by the roles of a Shared Area (ADR 0013) and won by latest wins together, and a text stays one Conflict unit (ADR 0011): two versions that differ only in their format are a Conflict, and resolving one writes text and format back together.

A value counts as marked when it is a JSON object whose `v` is missing or a whole number from 1, and, up to version 1, has a string `format` and a string `text`. Anything else reads as plain text, the whole value, as a text written before the format was recorded does; a value whose `v` is `"1"`, `1.5`, `0` or `null` reads so too. No release carried texts without a format, but reading them stays tolerant.

A Device edits a text only when its `v` is at most the version it knows (1) and it knows the `format`; otherwise it shows the source text read-only. Up to the known version, a key other than `format`, `text` and `v` makes the value not editable as well. A later version may add keys: in a value with a higher `v` they are ignored for display, `text` is shown when it is a string and nothing when it is missing or not a string, and the value is never editable here.

Names stay plain strings without a marker: Area, Topic and Named Place names, Profile names and Overrides. They are short, are never formatted, and their sanitising rules (ADR 0017) leave no room for markup. Transcripts and Photo Texts are machine text, kept next to the recording or photo, and stay plain strings as well.

## A format this version does not know

Such a text, and a text in a later structure, shows as its source, the `text` inside the value, read-only, with a note that a newer version of the app is needed to change it; on the phone and on the desktop alike. Whatever reads a text reads its source: cards, the Agenda and its widget, alarms and notifications, calendar events, Search, Sent Item previews.

Nothing here overwrites it:

- An edit of the Note's text is refused by both cores, and both editors show the text without a field to type into. On the phone no draft of it is kept or restored.
- Amend adds a new plain Section and Merge only points the merged Note at its target; neither writes the text.
- A Conflict with a version in a format not known here cannot be resolved on this Device: picking another version would overwrite it, and keeping it would mean choosing something the Device cannot show as it is. Both screens say so and offer no choice; a Device that knows the format resolves it, and the pick reaches this one by sync.
- Sync, the shared folder, bundles and Backups carry entries as they are, so the value arrives and leaves byte for byte.

## Sent Items

A Sent Item (ADR 0015) carries the format of the Note's text and of every Section's text next to the text, and the version of its structure in `v` when it is more than 1: `"note":{"format":…,"sections":[{"captured":…,"format":…,"text":…,"v":…}],"text":…,"v":…}`. A file without `format` reads as plain and one without `v` as version 1; a `format` that is not a string, or a `v` that is not a whole number from 1, makes the file unreadable. Accepting writes each text in the format and version it came in, so a text in a format not known here arrives whole. An update replaces the receiver's text only as before, and in addition never replaces a text in a format not known here unless it is the text the sender sent last: what a later version of the receiver's own wrote stays, while the sender's own formatted text follows the sender's updates.

## Formatted text (ADR 0033)

Amended by ADR 0033 (#127): this version knows two formats, `plain` and `markdown`, both in structure version 1, and edits a text in either. A plain text becomes `markdown` at its first format and stays so; a Device of 0.1.x, which knows only `plain`, keeps a formatted text read-only as this section describes. Sent Items, the shared folder, bundles and Backups carry `markdown` as they carry any format.

## Other values (ADR 0034)

Amended by ADR 0034 (#131): the same rule holds beyond texts. A Routine's schedule, a grant in a Shared Area, a Profile record and an Override carry the version of their structure, and a value of a later version is kept read-only with the same note, on the phone and on the desktop. Unlike a text, a value whose version is no whole number from 1 counts as of a later version too, as there is no source to show for it.

## Considered Options

- The format as a field of its own next to `text` (rejected: text and format could be won by latest wins from different writers, and a text would become a Conflict unit of two fields).
- A prefix inside the text, such as a header line (rejected: any prefix could also be text a user typed).
- No marker for plain text, only for formatted text (rejected: a later version could never tell plain text from text written before it chose its encoding).
- No structure version, a later version adding a new format name instead of a key (rejected: a later version could not add a key, such as one for attachments of a text, without a new name for every format it applies to; `v` says what a Device of this version may change).
- Writing `"v":1` on every text (rejected: a default is never written, and the texts already stored stay as they are).

## Consequences

- Every Note and Section text in the log is about 30 bytes longer; every fixture with texts changed.
- A Conflict over a text in a format not known here stays open on this Device until a newer Device resolves it.
- A sender's update whose text the receiver's later version had formatted keeps the receiver's text; the Sections and the Routines of the update still apply.
- Nothing changes for the user of this version: the note about a newer version shows only once a later version writes another format.
