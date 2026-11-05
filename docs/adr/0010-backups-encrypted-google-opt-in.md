# Encrypted Backups, opt-in Google backup holding only the encrypted file

Besides sync, a Device can write encrypted Backup files by hand or on a schedule into a folder of the user's choice. Android Auto Backup to the user's Google account is opt-in; because Android has no runtime switch for it, a custom backup agent backs up nothing until the user opts in, and even then Google only receives the encrypted Backup file, never the database. Forgetful users lose passphrases, so setup produces a Recovery Sheet (PDF with a QR code), and any still-paired Device can show the backup key again after its own unlock.

## Considered Options

- Backing up the plain database to Google (rejected: Google could read health notes).
- No recovery for a lost passphrase (rejected: the target users would lose everything).

## The iPhone (#244)

The iPhone's own backup to iCloud or a computer gets none of the app's data: on every start, before the database opens, the app leaves its Application Support (the database, the photos and recordings beside it, the alarm layer's state) and the App Group's folders (the Agenda widget's snapshot, the shares waiting for the app) out of it with `isExcludedFromBackup` (`ios/Runner/BackupExclusion.swift`). iOS has no opt-in backup agent like Android's; the encrypted Backup files go into the folder the user picks, which iCloud Drive can carry.

### Considered Options (#244)

- Keeping the database in the iPhone's backup (rejected: an iCloud backup or an unencrypted computer backup would hold the plain database, which Google never gets on Android; the app has its own encrypted Backups).

### Consequences (#244)

- A new iPhone set up from an iCloud or computer backup starts Old Brain Fix empty: it is brought back from a Backup file or by pairing with another Device.
