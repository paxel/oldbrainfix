# Removing a recording or photo keeps its text; the bytes go wherever nothing refers to them

A voice recording with a Transcript, or a photo with a Photo Text, can be removed while the text stays (#77). Removing is an edit to the Note: `audio:<hash>` or `photo:<hash>` set to null, a version 1 entry with no key of its own, signed, synced, judged by the roles of a Shared Area (ADR 0013) and won by latest wins like any other. The Transcript (`transcript:<hash>`) or Photo Text (`ocr:<hash>`) is not touched, so the Note keeps showing it, Search keeps finding it, and the Note lists the hash among its removed photos or recordings (`removedPhotos`, `removedRecordings`) for the screens to show the text without the file.

Both cores offer it as `mayRemovePhoto` / `mayRemoveRecording` and `removePhoto` / `removeRecording` on Notes (`may_remove_photo` … in Rust), for a Note or a merged Note shown as a Section. They refuse a recording or photo without a text, or with an empty one, which means nothing was found. They also refuse it on a Note this Device may not change (`mayChange`): in no Area or one it manages, or as a Member only on Notes they own. A Member's removal on another person's Note, written all the same, is ignored on every Device as any other edit of theirs is.

The bytes go once the log no longer refers to them. A blob is let go when every field that ever named its hash, on any Note, was last set to null by a removal that may let it go (`removedBlobs`). That is a removal written by one of the user's own Devices (this one, one paired as its own, or one whose key is bound to the own Profile), or one on a Note that lies in a Shared Area now, where the judge checked the writer's role. Each Device drops those bytes after its own removal and after every sync that brings anything (`dropRemovedBlobs`), and a folder sync deletes their sealed copies from the shared folder, under every key it holds. A photo shown on another Note too keeps its bytes until the last one lets it go. The sweep reads the whole log each time, so a judgement that changes later (a grant or a filing arriving after the removal) is caught at the next sync. A blob that was dropped too early is missing again once the judgement shows it, and is fetched from a Device that still holds it.

A removed blob is no longer wanted (`missingBlobs`), published or taken from another Device: `knowsBlob`, which decides whether pushed bytes are kept, now asks whether the log refers to a blob now, not whether it ever did.

## Older versions

An older version reads the null as every version reads it: the hash is not among the Note's photos or recordings, so it no longer shows the file, does not ask for its bytes and does not publish them. Search still finds the text. Its screens, which draw a Transcript or Photo Text only under its recording or photo, do not show the text on the Note until it is updated. It never deletes blobs, so it keeps the bytes on disk; once updated, its next sync that brings anything lets them go.

## Considered Options

- A new value such as `"removed"` instead of null (rejected: every older version would go on asking for the bytes and publishing them).
- A key or field of its own next to `photo:<hash>`, or an entry of a higher `v` (rejected: an older version would show a file it can no longer get, or not apply the removal at all).
- Dropping the bytes on every removal that counts in the projection (rejected: outside a Shared Area another person's Device is not refused (ADR 0013), and bytes once dropped are gone on every Device. Such a removal hides the file, which stays recoverable).
- Removing a recording or photo without a text (rejected by the ticket: the Note would lose what it said).

## Consequences

- Backups made before the removal still hold the bytes; restoring one restores the state it was made in.
- A file another person removed outside any Shared Area is hidden, not deleted: its bytes stay on this Device.
- A sync with one of the user's Devices met for the first time takes its removals but keeps the bytes until the next sync that brings anything: while the entries are taken in, that Device does not count as one of the user's own yet.

## Blocks (ADR 0021)

Amended by ADR 0021 (#90): blocking a person deletes the bytes of their photos and recordings that nothing else in the log refers to, with no removal entry; their copies in the shared folder stay.

## The text goes into the Note's text (obf-1084)

Amended by obf-1084: a Transcript or Photo Text kept beside its removed file could never be removed. Removing a recording or photo now keeps its words as an ordinary paragraph of the Note's own text, edited or deleted as typed text is, and leaves no Transcript or Photo Text. `removeRecording` / `removePhoto` (`remove_recording` / `remove_photo`) write three entries in one batch, in this order: the Note's text (`edit`, a conflictable edit with its causal context, ADR 0011) with the words after the text, a blank line between, or as the whole text when it is blank, escaped into a formatted text as `takeOver` escapes them (ADR 0025, ADR 0033); `transcript:<hash>` or `ocr:<hash>` set to null; then `audio:<hash>` or `photo:<hash>` set to null. The hash is then among neither the removed recordings or photos nor the removed Transcripts or Photo Texts, and search finds the words in the text. The bytes go as before; `removedBlobs` reads only the blob fields. For a merged Note the words go into its own text, which its Section shows.

`mayRemoveRecording` / `mayRemovePhoto` also refuse a Note whose text is in a format this version does not know (ADR 0018), as `edit` would. Nothing is written then, and the file and its text stay as they are.

Two Devices removing files of one Note before they sync each edit the text, so the text is a Conflict (ADR 0011) whose versions each hold one paragraph; none is lost, and the person picks. An older version reads the new entries as an edited text and two removed fields: it shows the words in the text and no Transcript or Photo Text. A Transcript or Photo Text another Device writes for the file before it learns of the removal can win over the null; the Note then holds the words in its text and again a text beside the removed file, as before obf-1084.

Notes whose file was removed before obf-1084 are left as they are: their text stays beside the Note, listed among `removedRecordings` / `removedPhotos` and shown, found and named by as before. Nothing moves it into the text on its own.

Attached files are not changed: removing one still keeps its Attachment Text beside the Note, since that text can be a whole document.

## Attached files (ADR 0047)

Amended by #267: an attached file is removed as a photo is: `file:<hash>` set to null, refused unless the file has an Attachment Text that is not empty, which stays and is still found, and its bytes go once nothing in the log names them (`mayRemoveAttachment`, `removeAttachment`; `may_remove_attachment`, `remove_attachment`). `file` is a blob kind beside `photo` and `audio`, so `removedBlobs` and `dropRemovedBlobs` let its bytes go by the same rules. The Note lists such a file among its removed ones with the name and type it had (`removedAttachments`), read from the last value its field held.

## Emptying the trash (obf-1095)

Amended by obf-1095: "Empty the trash now" (`Trash.empty`; `Trash::empty` in Rust) purges what is in the trash at once, as the 30-day purge does once the 30 days have passed, in one batch: each Note in the trash that is not merged and that this Device may change (`mayChange`) gets its `$type` set to null, its Sections and the Notes merged into it first where it may change them, then each Routine or Reminder in the trash it may change (`mayChangeRoutine`). Frogs stay (ADR 0027). Notes this Device may not change in a Shared Area stay in the trash, as the 30-day purge leaves them. Only what is in the trash in the log goes, in both purges: a Note or Routine an AI Task, or another pending set shown on the screens, only drafted into the trash (ADR 0022) stays, so that rejecting the set brings it back. `Trash.toEmpty` (`Trash::to_empty`) names the same Notes and Routines beforehand, for the question the apps ask.

The log keeps every entry, so sync is unchanged, and no removal is written: after the batch, the bytes of every photo, recording and file those entities named leave this Device only, unless a Note that was not purged still names the same hash. The user's other Devices keep theirs until they empty their own trash.

A blob field on a Note whose `$type` is null no longer counts as one the log refers to: `missingBlobs` does not want it back from another Device and `knowsBlob` refuses its bytes when they are pushed. Without that, the next sync would bring back what was let go. This holds for every purged Note, by the 30-day purge or a discard as well; the bytes those keep here stay where they are, as before, unless blocking a person or declining an offered Area, which let go of what the log does not refer to, takes them along. An older version still asks for such a blob; a Device that let it go answers that it lacks it, as for any blob it does not hold.

The AI port has no tool for it: an AI Task writes a type only as what the entity's id says it is (ADR 0026), and the purge cannot be undone.

Considered: setting every blob field of the purged Notes to null (rejected: a removal by the user's own Device lets the bytes go on every Device and shows in the Note's History, where "from this Device" was asked for).
