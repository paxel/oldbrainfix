# Every user text is stored with its format, and a format not known is never overwritten

Formatted text is planned for a later version (#57). So that a Device of this version cannot destroy formatted text written by a later one, this version stores the format of every text it writes, and leaves alone any text in a format it does not know (#58).

## The marker

The text of a Note and of a Section is stored as one value: `{"format":…,"text":…}`, keys sorted and no spaces, the same bytes from both cores (`text_format.dart`, `text_format.rs`). This version writes only `"format":"plain"`, the text as typed. Text and format sit in one entry, so they are signed, synced, judged by the roles of a Shared Area (ADR 0013) and won by latest wins together, and a text stays one Conflict unit (ADR 0011): two versions that differ only in their format are a Conflict, and resolving one writes text and format back together.

A value counts as marked when it is a JSON object with a string `format` and a string `text`. Anything else was written before the format was recorded and reads as plain text, the whole value. No release carried such texts, but reading them stays tolerant. A marked value is editable here only when its format is one this version knows and it holds no other key; a later version that needs more than the text adds a new format name instead of a key next to `plain`.

Names stay plain strings without a marker: Area, Topic and Named Place names, Profile names and Overrides. They are short, are never formatted, and their sanitising rules (ADR 0017) leave no room for markup. Transcripts and Photo Texts are machine text, kept next to the recording or photo, and stay plain strings as well.

## A format this version does not know

Such a text shows as its source, the `text` inside the value, read-only, with a note that a newer version of the app is needed to change it; on the phone and on the desktop alike. Whatever reads a text reads its source: cards, the Agenda and its widget, alarms and notifications, calendar events, Search, Sent Item previews.

Nothing here overwrites it:

- An edit of the Note's text is refused by both cores, and both editors show the text without a field to type into. On the phone no draft of it is kept or restored.
- Amend adds a new plain Section and Merge only points the merged Note at its target; neither writes the text.
- A Conflict with a version in a format not known here cannot be resolved on this Device: picking another version would overwrite it, and keeping it would mean choosing something the Device cannot show as it is. Both screens say so and offer no choice; a Device that knows the format resolves it, and the pick reaches this one by sync.
- Sync, the shared folder, bundles and Backups carry entries as they are, so the value arrives and leaves byte for byte.

## Sent Items

A Sent Item (ADR 0015) carries the format of the Note's text and of every Section's text next to the text: `"note":{"format":…,"sections":[{"captured":…,"format":…,"text":…}],"text":…}`. A file without `format` reads as plain; a `format` that is not a string makes the file unreadable. Accepting writes each text in the format it came in, so a text in a format not known here arrives whole. An update replaces the receiver's text only as before, and in addition never replaces a text in a format not known here unless it is the text the sender sent last: what a later version of the receiver's own wrote stays, while the sender's own formatted text follows the sender's updates.

## Considered Options

- The format as a field of its own next to `text` (rejected: text and format could be won by latest wins from different writers, and a text would become a Conflict unit of two fields).
- A prefix inside the text, such as a header line (rejected: any prefix could also be text a user typed).
- No marker for plain text, only for formatted text (rejected: a later version could never tell plain text from text written before it chose its encoding).

## Consequences

- Every Note and Section text in the log is about 30 bytes longer; every fixture with texts changed.
- A Conflict over a text in a format not known here stays open on this Device until a newer Device resolves it.
- A sender's update whose text the receiver's later version had formatted keeps the receiver's text; the Sections and the Routines of the update still apply.
- Nothing changes for the user of this version: the note about a newer version shows only once a later version writes another format.
