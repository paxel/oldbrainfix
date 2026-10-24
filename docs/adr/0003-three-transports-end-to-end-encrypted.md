# Three sync transports, end-to-end encrypted per Area, syncing automatically

Devices sync without a server over LAN (TLS, pairing by QR or code), a shared folder (Syncthing, Nextcloud, USB), and bundle files sent through any messenger. Unlike Catlog, where folder files and bundles are only signed, every Area has its own key, handed over at Pairing, and everything leaving a Device is encrypted with it; health notes on a third-party cloud or in a chat backup must stay unreadable. Unlike Catlog's manual LAN sessions, paired Devices find each other (mDNS) and sync on their own, in the background where the platform allows it, because nobody should have to remember to sync a memory aid.

## Consequences

- On Android, background sync runs at best about every 15 minutes, so a check-off on one Device can take that long to silence an alarm on another.
- Losing every copy of an Area's key makes that Area's synced files unreadable; Backups and the Recovery Sheet (ADR 0010) cover this.
