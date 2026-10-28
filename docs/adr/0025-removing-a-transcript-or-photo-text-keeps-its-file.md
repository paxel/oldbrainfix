# Removing a Transcript or Photo Text keeps its file; only the owner removes one

A Transcript or Photo Text is the machine's suggestion and stays read-only (#110). The user takes it over into the Note's own text instead of editing it: "Replace Note text" and "Append to Note text" are an edit of the Note's text (`edit`, ADR 0002), nothing of their own. The generated text can then be removed, and made again while its file is there.

## Removing

Removing is an edit to the Note, like removing a recording or photo (ADR 0020): `transcript:<hash>` or `ocr:<hash>` set to null, a version 1 entry with no key of its own, signed, synced and won by latest wins like any other. The recording or photo is not touched. Search no longer finds the text. The Note lists the hash among `removedTranscripts` or `removedPhotoTexts` (`removed_transcripts`, `removed_photo_texts`), and `untranscribed` and `unreadPhotos` leave it out, so no Device makes the text again on its own. Null now means removed; absent still means not made yet, and empty still means nothing was found.

Both cores offer `mayRemoveTranscript` / `mayRemovePhotoText` and `removeTranscript` / `removePhotoText` on Notes (`may_remove_transcript` … in Rust), for a Note or a merged Note shown as a Section. They refuse a text that is missing or empty, and the text of a recording or photo that was removed: that text is all the Note keeps of it. They also refuse it on a Note this Device may not change (`mayChange`).

Transcribing or reading again writes the new result with `setTranscript` / `setPhotoText` as before, which replaces the text and ends a removal. The apps offer it while the file is on this Device and the Note is this Device's to change.

## Roles

ADR 0013 lets a Member add Transcripts and Photo Texts to any Note, since the machine on any Device writes them. That now holds only for a text: a null under `transcript:` or `ocr:` is judged as any other edit of the Note, so a Member's removal on another person's Note is ignored on every Device. Writing a text over another one stays allowed to a Member at the level of the log, as before; the apps offer transcribing and reading again only where `mayChange` holds.

## Older versions

An older version reads the null as absent: the recording or photo has no text, so it does not show one and search does not find it, but it transcribes or reads the file again when it holds the bytes and the model, and the new text then reaches every Device. An older version also judges a Member's removal as a text a Member may add, so on it such a removal counts.

## Considered Options

- An empty text instead of null (rejected: a removal would read as "nothing found", and the judge could not tell a Member's machine finding nothing from a Member removing another person's text).
- Letting the user edit the generated text (rejected by the ticket: only the machine writes it; the Note's text is the user's).
- Removing the text of a removed recording or photo (rejected: nothing of the file would be left).

## Consequences

- A text removed on a Device with an older version still around can come back from that Device until it is updated.
- A removed text stays in the log, as an edited Note text does.
