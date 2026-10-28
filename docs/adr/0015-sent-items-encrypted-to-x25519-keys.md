# Sent Items: one signed file, encrypted to the recipient's X25519 keys when they are known

A Sent Item (ADR 0009) is a `.oldbrain` file, type `application/vnd.oldbrainfix.item`: one JSON object, so the app tells it from a sync bundle (a zip) by its first byte. It carries the Note's text, the texts and capture times of its Sections and the schedules of its Routines and Reminders (start, times, rule), under a Sent Item id, a version, the sender's signed Profile (ADR 0017, #51) and the sender's key record; never the Capture Context, photos or voice recordings, which keeps it small (at most 1 MiB is read) and private. The id is kept on the sender's Note as `sent:<device>` = `{"id":…,"version":…}`, so the same Device sending the same Note again sends the next version under the same id; another Device of the sender's sends under an id of its own, as only the same key may change a Sent Item.

The payload is signed as it is, keys sorted and no spaces, the same bytes from both cores: `{"format":1,"item":<payload as a JSON string>,"sig":<Ed25519 over "oldbrainfix:sent:" + payload>}`. The receiver checks the signature against the key record inside, whose device id is the key's fingerprint, then the Profile (ADR 0017): its own signatures, and that its binding holds the key that signed the file, or the file is unreadable. The preview shows the sender's name and picture and the Profile's short code, as verified when the user compared that code in person, known when the Profile is kept here, and new otherwise (#51).

To encrypt, every Device has an X25519 key of its own, derived from its Ed25519 seed with HKDF-SHA256 (info `oldbrainfix:box`), so there is no new secret to keep. Its public half travels in the key record as `box`, signed by the Ed25519 key as `boxSig` over `["box",device,box]`; the record's own signature stays as it was, and a pinned key without one takes it from a later record of the same key. A file to a known person is sealed with a random content key (XChaCha20-Poly1305, as ADR 0012, bound to `oldbrainfix:sent`), and the content key once per Device of theirs whose key is known: an ephemeral X25519 key, the shared secret through HKDF-SHA256 with both public keys as salt (info `oldbrainfix:sent-key`), sealing the content key: `{"format":1,"sealed":…,"to":[{"eph":…,"key":…}]}`. The stanzas name no Device; a receiver tries each. Inside lies the signed file, so who sent it is hidden too. Known people are those of the Shared Areas and the Devices paired by invite, never the Device's own group; to anyone else the file goes signed only, which the send dialog says.

Accepting makes a new Note of the receiver's, captured then and Unsorted, with the Sections and the Routines written as they are (not spawned, so the Note stays in Unsorted) and their alarms on. The Note keeps `received` = `{"from":<sender key>,"id":…,"profile":<sender's Profile id>,"version":…}` and each Routine the sender's id for it, in the log, so all of the receiver's Devices recognise a later version (#39). The same id from the same key is not accepted twice. Accepting also keeps the sender's Profile, as ADR 0017 keeps the Profiles of a Shared Area; declining keeps nothing, not the Profile either.

## Considered Options

- Turning the Ed25519 key into an X25519 one (rejected: one key pair for both signing and key agreement needs an argument of its own, and the Dart library does not convert keys; a separate key costs two fields).
- libsodium's sealed boxes (rejected: neither core's crypto library has them; HKDF and XChaCha20-Poly1305 are in both).
- Naming the recipient Device in each stanza (rejected: a file passed on would say whom it was for).
- Sending photos and voice recordings along (left out for now: a file to keep small, and what the receiver needs is the text and the times).

## Consequences

- Key records grew two fields, so every fixture carrying them changed; Devices paired before need one more sync before files to them can be encrypted.
- Sign-then-encrypt lets the recipient pass the signed inner file on, in the sender's name; it says only what the sender sent.
- A Sent Item from another Device of the sender's comes as a new one, not an update.

## Updates and cancellations

A later file with the same ID and a higher version is an update; a file carrying `"kind":"cancel"` is a cancellation (no `kind` means a normal item, any other kind is unreadable, so files from before stay byte-identical). Both are accepted only when signed by the same key as the item first accepted under that ID; a version at or below the one held writes nothing. Only a `received` link (and a Routine's sender id) that one of the user's own Devices wrote counts, the newest such one, as for the author shown (ADR 0017): in a Shared Area another person's Device can write any field of a Note, so a link it wrote reaches no Note. The same holds on the sending side for the `sent:<device>` link: only one that one of the user's own Devices wrote keeps a Sent Item's id, so a link another person's Device wrote makes the next send a new Sent Item, no update or cancellation of another.

## Profiles (ADR 0017)

Amended by ADR 0017: a Sent Item carries the sender's signed Profile instead of a name of its own; the preview shows it, and it is stored and bound to the sender's key only when the item is accepted. "Verified" moves from the sender's key to the Profile, set by comparing the Profile's short code in person (#51): a key met by pair code no longer makes a sender verified. An update or cancellation brings the sender's Profile again, and a newer record of it replaces the one kept, as any other does.

## Text formats (ADR 0018)

Amended by ADR 0018 (#58): the Note's text and every Section's text carry their format, `"format":"plain"` from this version, and the version of their structure as `v` when it is more than 1; a file without a format reads as plain, one without `v` as version 1. Accepting writes each text in its format, and an update never replaces a text in a format the receiver does not know unless it is the text the sender sent last.

## Blocks (ADR 0021)

Amended by ADR 0021 (#90): a Sent Item whose Profile id is blocked, or which a blocked person's Device signed, is refused before its preview and cannot be accepted. "Decline and block" keeps the sender's Profile without its picture.

## A stranger's Sent Item (ADR 0022)

Amended by #91: a Sent Item whose sender's Profile is not kept here (unknown, not verified or known) is held as Pending Changes, never shown (ADR 0022), until the user taps "Show". Until then the preview shows only the sender's name and that they are unknown: no text, Section, Routine or time, and not their picture either. Receiving the same item again finds the one held. Accept and Decline work before "Show" as after it: Accept checks the file again and judges it against the log as it is then, so the Note is captured at Accept and an item accepted meanwhile on another of the user's Devices writes nothing twice; Decline deletes the held file. A held item waits through a restart until it is decided. A Sent Item from a known or verified sender, or from the user's own Profile, previews as before.

## Profile cards (ADR 0017)

Amended for #107: known people also include every other person whose Profile is kept here and binds a key pinned here, unless they are blocked. A person added from a Profile card is one: the card pins the key of the Device that made it, so a Sent Item to them is sealed for that Device at once, and theirs previews as known, not held as a stranger's.
