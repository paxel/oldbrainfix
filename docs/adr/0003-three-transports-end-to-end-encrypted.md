# Three sync transports, end-to-end encrypted per Area, syncing automatically

Devices sync without a server over LAN (TLS, pairing by QR or code), a shared folder (Syncthing, Nextcloud, USB), and bundle files sent through any messenger. Unlike Catlog, where folder files and bundles are only signed, every Area has its own key, handed over at Pairing, and everything leaving a Device is encrypted with it; health notes on a third-party cloud or in a chat backup must stay unreadable. Unlike Catlog's manual LAN sessions, paired Devices find each other (mDNS) and sync on their own, in the background where the platform allows it, because nobody should have to remember to sync a memory aid.

## Consequences

- On Android, background sync runs at best about every 15 minutes, so a check-off on one Device can take that long to silence an alarm on another.
- Losing every copy of an Area's key makes that Area's synced files unreadable; Backups and the Recovery Sheet (ADR 0010) cover this.

## Every Device hosts (#105)

Amended for #105: phones and tablets host LAN sync as the computer does, so two phones on one Wi-Fi pair and sync without a computer. Either Device of a Pairing shows the Pairing QR, with the typed code under it, and the other scans or types it, in both directions; the computer types the code, it has no webcam. Own Devices pair only in person: the personal key and the own Profile travel only over a LAN session the pair code or a pinned certificate authenticated, never through a messenger, a folder or a bundle.

- Any Device hosts with a self-signed ECDSA P-256 certificate it makes once, `CN=oldbrainfix` with that name as its only subject alternative name, valid from when it was made with no end (RFC 5280's 9999-12-31); the desktop core makes it with `rcgen`, the Dart core writes the DER itself and signs with `pointycastle` (RFC 6979), since `cryptography` does not sign with P-256. Both keep it as local settings, `tls:cert` and `tls:key` (base64 DER, the key as PKCS#8 with its public key inside, as ring reads it); it is never synced and no Backup carries it. The pair code carries SHA-256 over the DER, as before.
- The pairing host takes the PIN of the code it shows; the paired host takes none and lets in only a paired Device that syncs on its own, proven by its key over a nonce from `GET /challenge` (two minutes, at most 256 held). The phone runs the paired host while the app is open and for the whole run of its background sync, on every IPv4 address and on the port it had before, and advertises it as the computer does: `_oldbrainfix._tcp`, the device id as the instance name and as the only TXT entry `id`. It hosts the code it shows on its Wi-Fi or Ethernet address, never on a mobile network's.
- A joiner names the whole fingerprint of its own certificate in `POST /sync` (`fingerprint`, 64 lower-case hex digits), and makes its certificate first when it has none. The host keeps it with the paired Device only when the body's `proof`, the joiner's signature over the host's id and the PIN (empty in a paired session), verifies under the key pinned for the device id the body names: anyone holding the PIN can name another Device's id, and a fingerprint kept for it would have this host join them later in that Device's name. So both Devices of a Pairing find and join each other on their own, whichever showed the code. A paired Device keeps the address it was found at for as long as its fingerprint stays the same.
- The desktop core joins by a typed code as the Dart core does (`join.rs`): the certificate must match the code before anything is sent, a host that invites it gets nothing of its own but its Profile, and it remembers the host with its whole fingerprint. It does not join on its own afterwards: it hosts for as long as it runs, and the phone joins it by the certificate the desktop named when it joined.
- The sync format stays 4: an older Device ignores the field, and its partner only does not learn its certificate.

### Consequences

- Two phones meet in the background only while one runs its background sync and hosts and the other's run looks for it; with Android's period of about 15 minutes this can take several periods. With the app open on either, they sync within a minute.
- Every Device that joined once holds a certificate, also one that never shows a code.
