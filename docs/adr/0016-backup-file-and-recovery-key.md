# Backup files: one backup key per Profile, wrapped by the passphrase, shown as the recovery key

A Backup (ADR 0010) is a `.obfbackup` file: one plain header line, then the payload sealed in chunks. Setting Backups up makes a random 256-bit backup key; the Recovery Sheet shows it as the recovery key, 52 Crockford base32 characters in groups of four (the key's 256 bits and 4 check bits from its SHA-256, so a typo is told apart from a wrong key), and as a QR of the same text. The passphrase, without the blanks around it, derives a key with Argon2id (m = 64 MiB, t = 3, p = 1, a random 16-byte salt; `cryptography`'s pure-Dart Argon2id and RustCrypto's `argon2` give the same bytes) that seals the backup key once (`pass`, bound to `oldbrainfix:backup-pass`). The Device keeps the backup key and that sealed copy as local settings, never the passphrase, so a scheduled Backup needs nobody to type anything, and the Recovery Sheet can be shown again at any time. The key belongs to the person's Profile (ADR 0017), not to the Device: one passphrase, one recovery key and one Recovery Sheet for all of a person's Devices, each of which writes Backup files of its own with it (#49, below).

Every file gets a random content key, sealed with the backup key (`key`, bound to `oldbrainfix:backup-key`). The header, keys sorted and no spaces in both cores, says `{"chunk":65536,"created":…,"format":1,"kdf":{"m":…,"p":…,"salt":…,"t":…},"key":…,"pass":…,"type":"oldbrainfix-backup"}`; restoring with the passphrase goes passphrase → backup key → content key, with the recovery key it starts at the backup key. The payload is cut into chunks of `chunk` bytes, each sealed with the content key (XChaCha20-Poly1305 as ADR 0012) bound to `oldbrainfix:backup:<SHA-256 of the header line>:<index>`, the last one, shorter than the others and empty when the payload ends on a chunk, with `:last` added: a changed header, a chunk moved, dropped or cut off, and a changed byte anywhere all fail.

The payload is a run of JSON lines, keys sorted: `about` (the format, the Device id and its signing seed), the own Profile as it goes to one's own Devices with the backup key's version (`{"backupVersion":n,"profile":…}`, ADR 0017), the playback speed chosen on the Device (`{"playbackSpeed":"1.5"}`, #86, below), every `entry` of the log as the folder writes it, in log order, every `moment` with its place in that order instead of its local `seq`, the Area `key`s, the `pinned` keys of other Devices, and every `blob` the log names as `{"blob":"photo/<hash>","size":n}` followed by its n raw bytes. Nothing is ever whole in memory but one blob; a record of a kind not known yet is skipped.

A Backup is restored only into an empty log, on the first start: on the phone before the Starter Set, on the desktop in a view of its own until it is answered once. The Device keeps the new identity it was installed with: a restored Device with the old one would write under the same device id and counters as the old Device if that still runs, and every Device would keep only one of each pair for good. It takes in the entries, Moments, blobs, Area keys and pinned keys, pins the old Device's key as verified, takes the Profile back, and keeps the backup key with its sealed copy and version, so the Recovery Sheet stays good. In every Shared Area where the old Device held a role, it signs, once and as the old Device, the same role for the new one, dated after every grant there, as a Device hands its role to one of its own group (ADR 0013); a lone Manager would otherwise lose the Area. Device settings (ADR 0008) stay behind, the playback speed aside (#86, below): alerts, Quiet Hours, theme, the App Lock and its PIN, the Area Locks, the paired Devices, the shared folder and the Backup folder; the new Device pairs again and picks its folders anew.

Backups go into a folder the user picks (a document tree on the phone, a directory on the desktop) as `oldbrainfix-<UTC time>-<first 8 digits of the device id>.obfbackup`, by hand or weekly by default, from the background work that also syncs; the last four of this Device are kept, those of other Devices in the same folder are left alone.

## Considered Options

- Restoring the old identity (rejected: two Devices writing under one id lose entries for good; the old phone may still be in a drawer, switched on).
- A new content key wrapped twice per file, by the passphrase and by the recovery key (rejected: a scheduled Backup would need the passphrase, or a key it unlocks kept on the Device, which comes to the same as keeping the backup key).
- The Device name in the plain header (rejected: the file may lie in a cloud folder, and with #41 at Google).
- Android Keystore for the backup key (rejected for now: the Area keys and the signing seed lie in the same database unwrapped; whoever reads it reads everything a Backup holds anyway).

## Consequences

- One Recovery Sheet per person; every Device of the person holds the backup key (below), so whoever reaches one of them reads the Backups of all of them.
- A changed passphrase would still open the Backups made before; changing it is left out for now.
- Sent Items sent before the restore can no longer be updated or cancelled; sending the Note again sends a new one (ADR 0015). Files encrypted to the old Device's X25519 key, updates and cancellations included, do not open on the new one until their senders met its key record in a sync.
- The other people of a Shared Area take the new Device's entries once they met it, or a Device of its group, in a sync (ADR 0013).
- The one role entry the restored Device signs as the old one collides with the old Device's next entry if that Device still runs; restoring is meant for a Device that is gone.
- Restoring writes the blobs before the log is committed: a file found damaged half-way leaves blobs nobody refers to, never a half log.

## Google's copy (#41)

On Android the app's backup agent (`AutoBackup.kt`) takes part in Auto Backup (`fullBackupOnly`) and backs up nothing until the user opts in on the Backups page, a Device setting (`backup:auto`, ADR 0008) mirrored to a flag the agent reads without Flutter. Opted in, the weekly Backup, or one written by hand, also goes into the app's own `files/google-backup/`, with nothing else kept there, and the agent hands Google that one file and nothing else, never the database; with it on, no folder is needed for weekly Backups. A file larger than the quota Google takes (25 MiB) is left out and the page says so. Restoring takes back only a Backup file into that directory. On the first start of a new phone, a Backup found there opens at once, asking for the passphrase or the recovery key; it is removed once restored, kept when the user starts without it. The opt-in does not come back with it.

### Considered Options (#41)

- The opt-in kept only as a file the agent looks for (rejected: the agent must not need Flutter, and a Backup file restored from Google must not count as opting in on the new phone).

### Consequences (#41)

- Google's copy is at most a week old, and only the newest one: Android keeps no history of it.
- A Backup larger than 25 MiB, photos and recordings counting, never reaches Google.

## One backup key per Profile (#49)

#40 and #41 made one backup key per Device and had each Device send its own to the Devices of its group, which kept it under the sender's id and showed it, after their own unlock, as that Device's recovery key. No release shipped that; it is replaced, without a migration, by one backup key per Profile (ADR 0017).

A Device sends the backup key where it sends the own Profile, and only there: as `backupKey` in the `POST /sync` body and the answer of a LAN session that is not with another person's Device, automatic sync included, and in its keys file in the shared folder, sealed with the personal key only, which bundles carry likewise; never in plain text, never to a Device paired by invite, nor while inviting or invited (ADR 0013). It is one JSON object, keys sorted: `{"key":…,"pass":{"m":…,"p":…,"salt":…,"sealed":…,"t":…},"profile":…,"version":n}`, the key in base64, its copy sealed by the passphrase, the Profile id it belongs to and its version. The sealed copy goes along so the receiver's Backup files open with the passphrase as well as with the recovery key. The receiver takes the Profile first, then the key: only one made for its own Profile as it stands after that take, and only one newer than its own, by a higher version, or at the same version by the larger key in base64, so Devices that set Backups up apart end with the same key. It keeps it as its own (`backup:key`, `backup:pass`, `backup:version`), and is then set up for Backups as if the user had set them up there; the Backups page shows the same Recovery Sheet. Setting Backups up makes version 1, setting them up again one more.

Pairing two Devices that each set up Backups before: the kept Profile's key wins. A Device that takes another Profile over ("Which one is you?", or its own still had the defaults, ADR 0017) sets the version of the key it held to 0. Any key the kept Profile has then replaces it on the next sync; when the kept Profile has none, the yielded key, still better than none, goes to the other Devices. While "Which one is you?" waits, no key moves, as the two Devices hold different Profiles.

A Backup carries the Profile as above. Restoring one takes the Profile back whole: record, signature, first key, binding, aliases and whether the user chose name and picture; a Profile the new Device made at its first start is replaced. The new Device's own key joins the binding, signed by the old Device's key, whose seed the Backup carries, as the Device hands its roles on (ADR 0013); without that, every change the new Device made to the Profile would be refused by the user's other Devices. The short code stays, as the first key does. The backup key keeps its version.

### Considered Options (#49)

- The key inside the Profile record or its signed parts (rejected: the Profile goes to other people from #50 on; the key must never).
- The newest key by clock time (rejected: two Devices' clocks need not agree; a version counted up and the bytes as the tie-break give the same answer everywhere).
- Dropping the key of a Device that takes another Profile over (rejected: when the kept Profile has no Backups, that Device's would stop without a word).
- Only the key, without the passphrase's sealed copy (rejected: the receiver's files would open with the recovery key only).

### Consequences (#49)

- A Device whose key gave way writes its next Backups with the kept key. Its older files, the last four of them until they are replaced, still open only with its own former passphrase or Recovery Sheet; that sheet opens nothing newer. Google's copy follows with the next Backup.
- The restored Device's key is bound to the Profile even when the old Device still runs.
- A Device that was not bound to the Profile when its Backup was written cannot bind the new one; the user's other Devices bind it when they next sync with it, as with any Device of their own.

## The playback speed (#86)

The playback speed of voice recordings (#78) is a Device setting of both cores (`playbackSpeed`, one of 0.5, 0.75, 1, 1.5 and 2), kept as text as the phone and the desktop have kept it since 0.1.1 ("1.0", "1.5"), so a speed chosen before carries over without a step. It is the one Device setting a Backup carries: a speed chosen here is written as its own record after the Profile, `{"playbackSpeed":"1.5"}`, the speed as text since the cores need not write fractions alike (ADR 0019); none is written while none is chosen. Restoring takes it only when it is text and one of the speeds offered; anything else is passed over and the new Device plays at normal speed. A version that does not know the record skips it as a record of a later kind, so the Backup format stays 1.

### Considered Options (#86)

- A general record of settings, a map of keys and values (rejected: restoring it would need a list of the keys it may set, so that a crafted file cannot plant a signing seed, a PIN or the backup key; one record per setting carried says what it may set by its name).
- The speed as a JSON number (rejected: the two cores would have to write each fraction alike for the file to stay the same byte for byte).
- Carrying every Device setting (rejected: alerts, Quiet Hours, locks and folders belong to the Device the user sets up, as above; the playback speed is a habit of the person listening).

### Consequences (#86)

- A Backup made before #86 restores at normal speed.
