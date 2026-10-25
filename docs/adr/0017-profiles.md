# Profiles: one per person, a signed record bound to its Devices' keys, kept beside the log

Every person has one Profile: an id, a name, a picture and a version. The first Device makes it with a random id (a new entity id, 32 hex digits) and its own signing key as the Profile's first key; a Device paired as one's own takes it over (#48). When two Devices that each have a Profile are paired as one's own, Pairing asks once "Which one is you?", showing both; the other id becomes an alias of the kept one, and Notes marked with it show the kept Profile without being written again (#48). Nothing about the Profile is asked at first start, at Capture or while sharing: the phone starts with its own name (`Settings.Global.DEVICE_NAME`, else its model), the computer with the login's full name and account picture, else its own name; the defaults follow the system until the user changes the name or the picture (#47), and only while no other Device holds the Profile (#48).

## The record (#47)

The record is one JSON object, keys sorted and no spaces, the same bytes from both cores: `{"id":…,"name":…,"picture":…,"version":n}`. `picture` is a JPEG of 256 × 256 pixels in base64 and is left out for the placeholder; `version` starts at 1 and counts up with every change of name or picture. The name is sanitised when it is typed, when a record is read, and when one is received: control characters, bidi controls and invisible characters (zero-width space, word joiner, soft hyphen, the Hangul fillers and the like, from a fixed list in both cores) are removed; a zero-width joiner or non-joiner stays only between two visible characters, a variation selector only right after one, and tag characters only inside a flag after U+1F3F4, so emoji and every script stay whole; then the name is trimmed, cut to 40 code points (no joiner or half a flag left at the end) and trimmed again. A name with nothing left is empty in the record, and every screen shows it as "No name" in its own language. Code points are counted, not graphemes, because the two cores' Unicode tables differ by version and must cut at the same place.

A picture is the square the user picks of a picture, re-encoded as a JPEG of 256 × 256 pixels (quality 85, transparent parts on white) with every metadata segment removed. A picture received is decoded and made again the same way, from its middle square, and refused when it is larger than 256 KB, declares an edge above 1024 pixels, or is no JPEG, PNG or WebP; a picture chosen on the Device may have edges up to 8192 pixels. Both limits are checked before anything is decoded. Both cores do this (`profiles.dart`, `profile.rs`); their JPEG encoders differ, so the same picture gives other bytes on the phone and on the desktop, and the fixtures carry pictures as data.

The placeholder shows the initials (the first character of the first and of the last word, upper case) in white on a circle whose colour is one of 12 fixed colours: the first byte of SHA-256 over `oldbrainfix:profile-colour:<id>`, modulo 12. The short code people compare in person is the first 40 bits of SHA-256 over `oldbrainfix:profile-code:<id>:` followed by the first key's 32 public bytes, in upper-case Crockford base32 as 4 + 4 characters (`4F2K-9QX7`): a copied id with another key gives another code.

## Where it lives (#47)

The own Profile is kept as local settings, not in the log: `profile` holds the record, `profile:first` the first key (base64), and `profile:nameChosen` and `profile:pictureChosen` say the user replaced a default. The log goes to one's own Devices and, Area by Area, to other people under Area keys (ADR 0012, 0013), but the Profile has to reach the people of every Shared Area and every Sent Item whatever their keys, as one signed unit that a newer version replaces whole; an append-only log would also keep every picture ever chosen. Like the key records (ADR 0015), the record travels inside what is sent, never as an entry. Nothing about it left the Device in #47; since #48 it goes to the user's own Devices, below.

## Signing and binding (#48, #50, #51)

A record is signed by a Device key bound to its Profile, over `oldbrainfix:profile:` followed by the record's bytes as sent; the receiver checks the signature before it reads anything else, keeps the record as received so it can pass it on unchanged, and shows the name sanitised and the picture made again. A Profile's binding starts at its first key; one's own new Devices join it through Pairing, each added by a key already bound, and the binding carries the first key, so every Device that holds the Profile computes the same short code. On first sight an id is bound to the keys that came with it; later, a record of a known id counts only when signed by one of its bound keys, and a newer version replaces the stored one, every view showing the new name and picture at once. Profiles travel with Shared Area sync, bundles and the shared folder (#48 between one's own Devices, #50 to the people of a Shared Area) and with Sent Items (#51).

### Between one's own Devices (#48)

The own Profile travels as one JSON object, keys sorted and no spaces, the same bytes from both cores: `{"aliases":[…],"first":…,"fresh":true,"keys":[…],"record":…,"sig":…,"signer":…}`. `record` is the record as a string, `signer` the public key that made `sig` over it, `first` the first key (keys in base64). `keys` lists the keys joined since, each `{"by":…,"key":…,"sig":…}`, signed by `by`, a key bound before it, over `oldbrainfix:profile-key:<id>:<key>`; a receiver binds them in a chain from the first key, and a key that only signed itself, or that no bound key signed, is no key bound. `aliases` lists the ids the Profile replaced, each `{"alias":…,"by":…,"sig":…,"to":…}`, signed over `oldbrainfix:profile-alias:<alias>:<to>` by a key bound to the Profile given up. `fresh` is there while the Profile still has the defaults on the sending Device (neither name nor picture chosen); it goes to one's own Devices only and is not signed.

It goes only where the backup key goes (ADR 0016): in the `POST /sync` body and the answer of a LAN session that is not with another person's Device, automatic sync included, and in the keys file in the shared folder, sealed with the personal key only, never an Area key, which bundles carry likewise. A Device keeps it as local settings beside the record: `profile:signed` (the signature and signer), `profile:keys`, `profile:aliases`, `profile:shown` (a received picture made again) and `profile:asked`. The sync format went to version 4 so older builds refuse cleanly; the manifest and the keys file keep their formats, as an older reader skips the field it does not know.

In every such session, and for every keys file it opens, a Device whose key is bound to its own Profile first binds the other Device's key to it, signed with its own: the session or the personal key shows that Device is one of the user's own. Then it takes the Profile sent:

- The same id: the first key must be the same (a copied id with another key is refused); keys joined and aliases signed by a bound key are added; the record counts only when its signer is bound, and replaces the stored one when its version is higher, or at the same version when its bytes are larger, so two Devices that changed it apart end with the same one. A name or picture changed elsewhere counts as chosen here.
- Another id that names this Device's id as its alias, signed by a key bound here: this Device takes it over.
- Another id that this Device gave up: nothing changes, unless both gave each other up, answered apart; then the lower id stays on both.
- Another id otherwise: a Device whose Profile still has the defaults takes over one that has none, without a question; when both have them, the lower id stays. When the user changed both, "Which one is you?" waits on both Devices, showing the two Profiles with picture, name and short code; on the phone it opens right after Pairing and stays on the Profile page, on the computer it opens over the window and stays on the Profile page when put off. The first answer settles it: keeping this Device's Profile makes the other id its alias, taking the other one over makes this one's id the other's alias; the next sync carries it, and the other Device then takes the kept Profile over or finds its own question answered.

Taking a Profile over keeps its record, signature, first key, keys and aliases, adds this Device's former id as an alias signed with its own key, and keeps the other's name and picture as chosen unless they were still the defaults. From then on the Profile follows no Device's defaults: once another Device holds it, a phone's name and a computer's login would replace each other's back and forth. A Device that has no Profile yet takes the first one it is sent.
## Overrides and People (#52, #53)

An Override is a name and picture one person gives another's Profile for themselves alone ("Mum"), sanitised like a Profile. Overrides sync between one's own Devices and never leave them: they are kept in the log outside every Area, under the personal key (ADR 0012), which no other person holds. A person's picture opens their Profile, and the Profile page lists every stored Profile under People (#52); "Their Notes" there opens the browser filtered to that person (#53).

## Backup key per Profile (#49)

One backup key per Profile instead of one per Device: one passphrase and one Recovery Sheet for all of a person's Devices. A Device paired as one's own receives the key over the encrypted paths ADR 0016 uses for a Device's own key; each Device keeps writing Backup files of its own, all opened with the same key, and restoring one restores the Profile.

## The Profile page (#47)

The settings gear in the Agenda's corner is replaced by the own Profile's picture, on the phone and on the desktop. It opens the Profile page: the picture, the name and the short code; Backups and recovery; Your Devices (pairing and the paired Devices, as the Sync page shows them); People (#52); and the settings of this Device, which stay unsynced (ADR 0008).

## Changes to earlier decisions

- ADR 0013: the name a Manager types for an invited person is replaced by the person's Profile, and Notes are marked by Profile through the signing Device's binding and the aliases (#50).
- ADR 0015: a Sent Item carries the sender's signed Profile, stored and bound only when the item is accepted; "verified" moves from the Sent Item's key to the Profile, by comparing short codes in person (#51).
- ADR 0016: the backup key belongs to the Profile, not to the Device; the per-Device keys and their hand-over between Devices of #40 and #41 go, and a Backup carries the Profile (#49).

## Considered Options

- Profile fields as log entries (rejected: they would reach other people only inside an Area's history, under its key, and every past picture would stay in the log for good).
- The first key inside the record (rejected: the record holds what a person changes; the key belongs to the binding, which says who may change it).
- An id derived from the first key, as device ids are (rejected: a Device that takes the Profile over would show another person; the id stays when keys join and go).
- Storing "No name" in the record (rejected: one language's words would travel to readers of another).
- Counting the 40 characters as graphemes (rejected: the cores' Unicode versions could cut the same name at different places).

## Consequences

- A Profile's name and picture follow the system only on the Device that made it, and only until another Device holds it; after that only the user changes them.
- A forgotten Device stays bound to the Profile; unbinding a key is left out, as key rotation is (ADR 0012).
- Answered on both Devices before they meet again, and differently, "Which one is you?" ends with the lower id on both.
- A change of name or picture alone makes no bundle: a bundle is written only when there are entries to send, and carries the Profile with them.
- Until #49 a Backup does not carry the Profile: a restored Device starts with the defaults again.
- The account picture is read where the system keeps it on Linux (AccountsService's icon or `~/.face`); inside the Flatpak, on Windows and on macOS the initials show until the user picks a picture. The name comes from the user database there too (passwd's full name, Windows' display name).
- A name cut at 40 code points may lose the end of a long emoji sequence, never the characters before it.
