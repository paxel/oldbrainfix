# Backup files: one backup key per Device, wrapped by the passphrase, shown as the recovery key

A Backup (ADR 0010) is a `.obfbackup` file: one plain header line, then the payload sealed in chunks. Setting Backups up makes a random 256-bit backup key; the Recovery Sheet shows it as the recovery key, 52 Crockford base32 characters in groups of four (the key's 256 bits and 4 check bits from its SHA-256, so a typo is told apart from a wrong key), and as a QR of the same text. The passphrase, without the blanks around it, derives a key with Argon2id (m = 64 MiB, t = 3, p = 1, a random 16-byte salt; `cryptography`'s pure-Dart Argon2id and RustCrypto's `argon2` give the same bytes) that seals the backup key once (`pass`, bound to `oldbrainfix:backup-pass`). The Device keeps the backup key and that sealed copy as local settings, never the passphrase, so a scheduled Backup needs nobody to type anything, and the Recovery Sheet can be shown again at any time.

Every file gets a random content key, sealed with the backup key (`key`, bound to `oldbrainfix:backup-key`). The header, keys sorted and no spaces in both cores, says `{"chunk":65536,"created":…,"format":1,"kdf":{"m":…,"p":…,"salt":…,"t":…},"key":…,"pass":…,"type":"oldbrainfix-backup"}`; restoring with the passphrase goes passphrase → backup key → content key, with the recovery key it starts at the backup key. The payload is cut into chunks of `chunk` bytes, each sealed with the content key (XChaCha20-Poly1305 as ADR 0012) bound to `oldbrainfix:backup:<SHA-256 of the header line>:<index>`, the last one, shorter than the others and empty when the payload ends on a chunk, with `:last` added: a changed header, a chunk moved, dropped or cut off, and a changed byte anywhere all fail.

The payload is a run of JSON lines, keys sorted: `about` (the format, the Device id and its signing seed), every `entry` of the log as the folder writes it, in log order, every `moment` with its place in that order instead of its local `seq`, the Area `key`s, the `pinned` keys of other Devices, and every `blob` the log names as `{"blob":"photo/<hash>","size":n}` followed by its n raw bytes. Nothing is ever whole in memory but one blob; a record of a kind not known yet is skipped.

A Backup is restored only into an empty log, on the first start: on the phone before the Starter Set, on the desktop in a view of its own until it is answered once. The Device keeps the new identity it was installed with: a restored Device with the old one would write under the same device id and counters as the old Device if that still runs, and every Device would keep only one of each pair for good. It takes in the entries, Moments, blobs, Area keys and pinned keys, pins the old Device's key as verified, and keeps the backup key with its sealed copy, so the Recovery Sheet stays good. In every Shared Area where the old Device held a role, it signs, once and as the old Device, the same role for the new one, dated after every grant there, as a Device hands its role to one of its own group (ADR 0013); a lone Manager would otherwise lose the Area. Device settings (ADR 0008) stay behind: alerts, Quiet Hours, theme, the App Lock and its PIN, the Area Locks, the paired Devices, the shared folder and the Backup folder; the new Device pairs again and picks its folders anew.

Backups go into a folder the user picks (a document tree on the phone, a directory on the desktop) as `oldbrainfix-<UTC time>-<first 8 digits of the device id>.obfbackup`, by hand or weekly by default, from the background work that also syncs; the last four of this Device are kept, those of other Devices in the same folder are left alone.

## Considered Options

- Restoring the old identity (rejected: two Devices writing under one id lose entries for good; the old phone may still be in a drawer, switched on).
- A new content key wrapped twice per file, by the passphrase and by the recovery key (rejected: a scheduled Backup would need the passphrase, or a key it unlocks kept on the Device, which comes to the same as keeping the backup key).
- The Device name in the plain header (rejected: the file may lie in a cloud folder, and with #41 at Google).
- Android Keystore for the backup key (rejected for now: the Area keys and the signing seed lie in the same database unwrapped; whoever reads it reads everything a Backup holds anyway).

## Consequences

- One Recovery Sheet per Device: the backup key never travels, so a paired Device cannot show another's recovery key yet (ADR 0010 promises it; left for later).
- A changed passphrase would still open the Backups made before; changing it is left out for now.
- Sent Items sent before the restore can no longer be updated or cancelled; sending the Note again sends a new one (ADR 0015). Files encrypted to the old Device's X25519 key, updates and cancellations included, do not open on the new one until their senders met its key record in a sync.
- The other people of a Shared Area take the new Device's entries once they met it, or a Device of its group, in a sync (ADR 0013).
- The one role entry the restored Device signs as the old one collides with the old Device's next entry if that Device still runs; restoring is meant for a Device that is gone.
- Restoring writes the blobs before the log is committed: a file found damaged half-way leaves blobs nobody refers to, never a half log.
