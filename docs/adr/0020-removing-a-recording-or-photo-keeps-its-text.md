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
