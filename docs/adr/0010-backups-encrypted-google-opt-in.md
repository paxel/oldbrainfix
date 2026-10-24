# Encrypted Backups, opt-in Google backup holding only the encrypted file

Besides sync, a Device can write encrypted Backup files by hand or on a schedule into a folder of the user's choice. Android Auto Backup to the user's Google account is opt-in; because Android has no runtime switch for it, a custom backup agent backs up nothing until the user opts in, and even then Google only receives the encrypted Backup file, never the database. Forgetful users lose passphrases, so setup produces a Recovery Sheet (PDF with a QR code), and any still-paired Device can show the backup key again after its own unlock.

## Considered Options

- Backing up the plain database to Google (rejected: Google could read health notes).
- No recovery for a lost passphrase (rejected: the target users would lose everything).
