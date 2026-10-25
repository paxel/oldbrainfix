# Sent Items: one signed file, encrypted to the recipient's X25519 keys when they are known

A Sent Item (ADR 0009) is a `.oldbrain` file, type `application/vnd.oldbrainfix.item`: one JSON object, so the app tells it from a sync bundle (a zip) by its first byte. It carries the Note's text, the texts and capture times of its Sections and the schedules of its Routines and Reminders (start, times, rule), under a Sent Item id, a version, the sender's name and the sender's key record; never the Capture Context, photos or voice recordings, which keeps it small (at most 1 MiB is read) and private. The id is kept on the sender's Note as `sent:<device>` = `{"id":…,"version":…}`, so the same Device sending the same Note again sends the next version under the same id; another Device of the sender's sends under an id of its own, as only the same key may change a Sent Item.

The payload is signed as it is, keys sorted and no spaces, the same bytes from both cores: `{"format":1,"item":<payload as a JSON string>,"sig":<Ed25519 over "oldbrainfix:sent:" + payload>}`. The receiver checks the signature against the key record inside, whose device id is the key's fingerprint, and shows the sender as verified when that key was pinned here in person, known when it came with a sync, and never met otherwise, always with its key code.

To encrypt, every Device has an X25519 key of its own, derived from its Ed25519 seed with HKDF-SHA256 (info `oldbrainfix:box`), so there is no new secret to keep. Its public half travels in the key record as `box`, signed by the Ed25519 key as `boxSig` over `["box",device,box]`; the record's own signature stays as it was, and a pinned key without one takes it from a later record of the same key. A file to a known person is sealed with a random content key (XChaCha20-Poly1305, as ADR 0012, bound to `oldbrainfix:sent`), and the content key once per Device of theirs whose key is known: an ephemeral X25519 key, the shared secret through HKDF-SHA256 with both public keys as salt (info `oldbrainfix:sent-key`), sealing the content key: `{"format":1,"sealed":…,"to":[{"eph":…,"key":…}]}`. The stanzas name no Device; a receiver tries each. Inside lies the signed file, so who sent it is hidden too. Known people are those of the Shared Areas and the Devices paired by invite, never the Device's own group; to anyone else the file goes signed only, which the send dialog says.

Accepting makes a new Note of the receiver's, captured then and Unsorted, with the Sections and the Routines written as they are (not spawned, so the Note stays in Unsorted) and their alarms on. The Note keeps `received` = `{"from":<sender key>,"id":…,"name":…,"version":…}` and each Routine the sender's id for it, in the log, so all of the receiver's Devices recognise a later version (#39). The same id from the same key is not accepted twice; declining keeps nothing.

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

A later file with the same ID and a higher version is an update; a file carrying `"kind":"cancel"` is a cancellation (no `kind` means a normal item, any other kind is unreadable, so files from before stay byte-identical). Both are accepted only when signed by the same key as the item first accepted under that ID; a version at or below the one held writes nothing.

## Profiles (ADR 0017)

Amended by ADR 0017: a Sent Item carries the sender's signed Profile instead of a name of its own; the preview shows it, and it is stored and bound to the sender's key only when the item is accepted. "Verified" moves from the sender's key to the Profile, set by comparing the Profile's short code in person (#51).
