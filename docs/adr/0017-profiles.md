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

The own Profile travels as one JSON object, keys sorted and no spaces, the same bytes from both cores: `{"aliases":[…],"first":…,"fresh":true,"keys":[…],"record":…,"sig":…,"signer":…}`. `record` is the record as a string, `signer` the public key that made `sig` over it, `first` the first key (keys in base64). `keys` lists the keys joined since, each `{"by":…,"key":…,"sig":…}`, signed by `by`, a key bound before it, over `oldbrainfix:profile-key:<id>:<key>`; a receiver binds them in a chain from the first key, and a key that only signed itself, or that no bound key signed, is no key bound. `aliases` lists the ids the Profile replaced, each `{"alias":…,"by":…,"sig":…,"to":…}`, signed over `oldbrainfix:profile-alias:<alias>:<to>` by a key bound to the Profile given up, sorted by alias and then by key. An entry is kept by its alias and its key together, so an id can be listed once for each key that made it; a list only grows. The entry "Keep mine" makes is signed with the answering Device's key, which the other Device bound to the Profile given up when they met; it tells one's own Devices which Profile is kept, but other people may not know that binding. The Device that gives its id up therefore signs it again with its own key when it takes the kept Profile over, and that entry is the one other people count. `fresh` is there while the Profile still has the defaults on the sending Device (neither name nor picture chosen); it goes to one's own Devices only and is not signed.

It goes only where the backup key goes (ADR 0016): in the `POST /sync` body and the answer of a LAN session that is not with another person's Device, automatic sync included, and in the keys file in the shared folder, sealed with the personal key only, never an Area key, which bundles carry likewise. A Device keeps it as local settings beside the record: `profile:signed` (the signature and signer), `profile:keys`, `profile:aliases`, `profile:shown` (a received picture made again) and `profile:asked`. The sync format went to version 4 so older builds refuse cleanly; the manifest and the keys file keep their formats, as an older reader skips the field it does not know.

In every such session, and for every keys file it opens, a Device whose key is bound to its own Profile first binds the other Device's key to it, signed with its own: the session or the personal key shows that Device is one of the user's own. Then it takes the Profile sent:

- The same id: the first key must be the same (a copied id with another key is refused); keys joined and aliases signed by a bound key are added; the record counts only when its signer is bound, and replaces the stored one when its version is higher, or at the same version when it has a picture and the stored one none (#107), else when its bytes are larger, so two Devices that changed it apart end with the same one. A name or picture changed elsewhere counts as chosen here.
- Another id that names this Device's id as its alias, signed by a key bound here: this Device takes it over.
- Another id that this Device gave up: nothing changes, unless both gave each other up, answered apart; then the lower id stays on both.
- Another id otherwise: a Device whose Profile still has the defaults takes over one that has none, without a question; when both have them, the lower id stays. When the user changed both, "Which one is you?" waits on both Devices, showing the two Profiles with picture, name and short code; on the phone it opens right after Pairing and stays on the Profile page, on the computer it opens over the window and stays on the Profile page when put off. The first answer settles it: keeping this Device's Profile makes the other id its alias, taking the other one over makes this one's id the other's alias; the next sync carries it, and the other Device then takes the kept Profile over or finds its own question answered.

Taking a Profile over keeps its record, signature, first key, keys and the aliases a key bound to it made, adds this Device's former id as an alias signed with its own key, also when the Profile already lists that id under another key, and keeps the other's name and picture as chosen unless they were still the defaults. From then on the Profile follows no Device's defaults: once another Device holds it, a phone's name and a computer's login would replace each other's back and forth. A Device that has no Profile yet takes the first one it is sent.
### The people of a Shared Area (#50)

Another person's Profile is kept as a local setting `person:<id>`, as received with its signature and binding, so it can be passed on unchanged, with when it was first seen here and its picture made again. On first sight the id is bound to the first key and the keys joined to it in a chain; later a record of that id counts only when its first key is the same and one of its bound keys signed it, keys join only when a bound key signed them, aliases only when a bound key made them, and a newer version replaces the record (the same order as for the own Profile). Its name is sanitised and its picture made again on receipt, or the Profile is refused whole. The own Profile and its aliases are never taken from someone else.

The Profiles travel as a list `profiles` of signed Profiles, sorted by id, without `fresh`:

- In a LAN session with another person's Device, request and answer carry the own Profile and the Profiles of the people the grants of the Areas shared with that Device name now; the receiver takes only those whose id, or an id they replaced, those grants name, read after the session's entries. A phone joining a computer that invites it sends nothing but its own Profile (`profile`) for the computer to show; a phone inviting a computer fetches the computer's Profile with `GET /profile`, which a host answers to anyone holding the PIN. The invited Device's Profile must bind the key its device id is made from, or the invite is refused.
- Between one's own Devices, every Profile kept goes along, and every one is taken.
- In the shared folder each Device writes, for every Shared Area whose key it holds, `keys/<device>.<key id>.profiles`, sealed with that Area's key (`oldbrainfix:profiles:<device>` as associated data, so a keys file and a Profiles file cannot stand in for each other), holding the own Profile and those of the people the Area's grants name. Only the people of the Area hold its key, so only they read them; a reader takes those the grants name, after the entries. Bundles carry the same files. The nonces follow the keys files, by key id.

The sync format stays 4: no build that speaks 4 was released, so no older peer has to refuse the new fields.

A Note shows its writer's picture, or the placeholder, when another person wrote it: the Device that wrote its first `$type` is not this one, one of its own group, or bound to the own Profile. Its key leads to the Profiles that bind it; in a Shared Area the grant of that Device names the person, and the Profile shown is the kept Profile of that person, through the aliases, among those binding the key, so a Profile that binds somebody else's key shows none of their Notes. Until that Profile arrives, the placeholder of the named id shows. Outside a Shared Area the Profile first seen with the key shows. One's own Notes show no picture.

### Sent Items (#51)

A Sent Item carries the sender's own Profile as it goes to other people, `from` = `{"key":<key record>,"profile":<signed Profile>}` inside what is signed (ADR 0015). The receiver checks the Profile's signatures and that its binding holds the key that signed the file; a file whose Profile fails either is unreadable, so nobody sends in another person's name with a Profile copied from them. The preview shows the name sanitised, the picture made again and the short code; nothing of it is kept until the item is accepted, and declining keeps nothing. Accepting, an update or a cancellation takes the Profile in as a Shared Area's are taken (#50): kept on first sight and bound to its keys, later replaced by a newer record a bound key signed. The accepted Note keeps the Profile's id beside the sender's key in its `received` link, and shows the Profile bound to that key under that id, as another person's Note does; on the sender's own Devices, which hold the Profile, it shows none. The link counts only when one of the user's own Devices wrote it (this one, one paired as its own, or one whose key is bound to the own Profile), the newest such one: in a Shared Area a Member's Device can write any field of its own Notes and a Manager's of every Note, so a link another person's Device wrote marks nothing, and the Note shows the writer of its first `$type` as any other. The Note is one's own only when the key is bound to the own Profile, whatever id the link names; and the id counts only when a Profile kept here under it binds the key, so a Sent Item carrying one's own id or a third person's under another key shows the Profile first seen with that key.

"Verified" belongs to the Profile kept here: `"verified":true` in `person:<id>`, kept through every newer record. The preview offers "Compare codes", which shows the code the sender must read out from their Profile page; when the user says the codes match, the Profile is marked verified on accepting, and only when the code kept here for that id is the one compared, so a copied id with another first key verifies nobody. The preview shows the sender as verified when the kept Profile is, known when it is kept and binds the key that signed, and new otherwise; one's own Profile counts as verified.

## Overrides and People (#52, #53)

An Override is a name and picture one person gives another's Profile for themselves alone ("Mum"), sanitised like a Profile. Overrides sync between one's own Devices and never leave them: they are kept in the log outside every Area, under the personal key (ADR 0012), which no other person holds. A person's picture opens their Profile, and the Profile page lists every stored Profile under People (#52); "Their Notes" there opens the browser filtered to that person (#53).

An Override is the entity `override:<profile id>`, the id the person's Profile stands for through its aliases when it is written, with the fields `overrideName` (the name, sanitised, or null for none) and `overridePicture` (a Profile picture made the same way as one's own, a JPEG in base64, or null for none); field names of their own, so concurrent changes are never a Conflict. Lying in no Area, it is sealed under the personal key in the shared folder and bundles, goes with every sync between one's own Devices, and never with what goes to another person's Device, a Shared Area's segments or a Sent Item. Only entries written by one's own Devices count (this one, one paired as its own, or one whose key is bound to the own Profile): an Override another person's modified app slips into the log changes nothing. A name is sanitised again when read; a picture that is not standard base64 of a JPEG of at most 256 KB shows none. The own Profile takes none.

Every view shows the Override where set: other people's Notes, an Area's people, the People list, the Sent Item recipients and the preview of a Sent Item whose sender is kept here and bound to the key that signed it (a copied id shows its own name). The person's Profile shows their own name and picture, the short code, whether it is verified, "Compare codes" (marking it verified on a match, as for a Sent Item) and "Name and picture for me". A Sent Item's sender opens it only once their Profile is kept; one's own picture opens nothing.

## Backup key per Profile (#49)

One backup key per Profile instead of one per Device: one passphrase and one Recovery Sheet for all of a person's Devices. A Device paired as one's own receives the key, with its copy sealed by the passphrase, wherever it receives the own Profile, and takes it after the Profile, only for its own Profile and only when newer; each Device keeps writing Backup files of its own, all opened with the same key (ADR 0016).

When two Devices that each set up Backups are paired, the kept Profile's key wins: the Device that takes the other Profile over lets its own key give way, and keeps it only while the kept Profile has none. Its older Backup files still open with its former Recovery Sheet.

A Backup carries the own Profile with its binding. Restoring one restores the Profile on the new Device, with the aliases a key bound to it made, replacing one made there at first start; the new Device gets a new device identity (ADR 0016), and its key joins the binding signed by the old Device's key, which the Backup holds, so the short code stays and its changes to the Profile count on the user's other Devices.

## The Profile page (#47)

The settings gear in the Agenda's corner is replaced by the own Profile's picture, on the phone and on the desktop. It opens the Profile page: the picture, the name and the short code; Backups and recovery; Your Devices (pairing and the paired Devices, as the Sync page shows them); People (#52); and the settings of this Device, which stay unsynced (ADR 0008).

## Changes to earlier decisions

- ADR 0013: the name a Manager types for an invited person and the person id of the grant are replaced by the person's Profile id; the invite shows the other person's Profile to confirm, and Notes are marked by Profile through the signing Device's binding and the aliases, checked against the grant (#50).
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
- A restored Device is not the Device that made the Profile, so the Profile no longer follows any Device's defaults after a restore.
- The account picture is read where the system keeps it on Linux (AccountsService's icon or `~/.face`); inside the Flatpak, on Windows and on macOS the initials show until the user picks a picture. The name comes from the user database there too (passwd's full name, Windows' display name).
- A name cut at 40 code points may lose the end of a long emoji sequence, never the characters before it.
- A Profile is bound to the keys it came with when it is first seen, from whoever passed it on: a copied id with another first key that arrives first keeps the real one out on that Device. The short code, compared in person, tells them apart.
- "Verified" is a local setting of the Device where the codes were compared; the user's other Devices do not learn it.
- A key met by pair code no longer makes a Sent Item's sender verified; only the short code compared does.
- A person removed from a Shared Area keeps its key (ADR 0012, 0013), so they can still read the Profiles published in the shared folder under it afterwards.

## Blocks (ADR 0021)

Amended by ADR 0021 (#90): a blocked person's Profile is kept with its picture taken out of the record (`"stripped":true`), never passed on, and no newer record of theirs is taken while they are blocked; after an unblock any record of theirs at the same version or newer replaces it. People leaves blocked people out. The block itself lies in the log like an Override, `block:<profile id>`, and counts only when one of the user's own Devices wrote it.

## A Device of one's own added at first start (#106)

The first start asks first whether this is a new user, a Device of one's own being added, or a Backup being restored, and the Profile is made with the defaults only after the answer: at once for a new user, after the restore or the first sync otherwise, where it is already there and the defaults change nothing. A Device being added therefore holds no Profile when it pairs, and takes over the one the other Device sends in that first sync as a Device without a Profile does, whichever id is lower, with the backup key after it, as a joiner or as the host. Made at once, its fresh Profile would meet a Profile that is usually still on the defaults as well, and the lower of the two ids would stay, so in about half the cases the Device holding everything would take over the new Device's Profile and its name. A cancelled or failed Pairing leaves the Device without a Profile and back on the first screen.

## Profile cards (#107)

Amended by #144, below: a card is sent as a file, with the picture; the link remains only what the QR holds.

A person can add another without meeting them or a computer: each sends their own Profile card through any messenger, or shows it as a QR. Both cores make and open cards the same way (`profile_cards.dart`, `cards.rs`).

A card is the link `https://paxel.github.io/oldbrainfix/p#<payload>.<signature>`, both parts base64url without padding; what follows the `#` never reaches a server. The payload is one JSON object, keys sorted and no spaces, the same bytes from both cores: `{"back":true,"format":1,"key":<key record>,"profile":<signed Profile>}`. `key` is the key record of the Device that made the card, with its X25519 key (ADR 0015); `profile` is the own Profile as it goes to other people, its record without the picture and signed again by that Device, with the binding and no aliases; `back` is there only on a card made to answer one the user opened. The signature is that Device's, over `oldbrainfix:profile-card:` followed by the payload as sent, so a card changed anywhere, its flag included, reads as no card. The picture stays out because a 256 × 256 JPEG does not fit a QR; a one-Device Profile with a 40-code-point name makes a link of under 1,300 characters.

Opening a card (an App Link tapped, a QR scanned, a link pasted on the computer) reads only the printable ASCII of the text, takes what follows the last `#` up to the first character no card has, without a full stop after it, and reads at most 16 KiB of that. It then checks, before anything is kept: the key record is valid, its key made the card's signature, the Profile's own signatures hold, and its binding holds the card's key. The person is then taken in as another person's Profile is (#50), not verified, and the card's key is pinned on trust as any key a payload carries, so Sent Items to them are sealed for that Device (ADR 0015). Nothing is kept from a card of the user's own Profile, of a blocked person, or made by a Device a grant names for a blocked person (ADR 0021); a card naming an id kept here under another first key is refused, as a copied id is everywhere else. A card scanned in person marks the person verified at once, through the short code the card's first key gives, so a copied id verifies nobody; one opened from a link stays not verified until the user compares codes on their Profile. "Send yours back" is offered when the person was new here and the card is not itself an answer; the card sent back carries `back`, so the two do not ask each other again.

A card adds a person and nothing else: no Area, no grant, no Pairing, no key but the public ones; neither `fresh` nor anything else only one's own Devices are told goes into it.

At the same version, a record with a picture now replaces one without, before the larger bytes decide, for the own Profile and for other people alike. A card's record keeps the version of the record it was made of, and `"version"` sorts after `"picture"`, so by bytes alone the card's record would win over the full one wherever both arrive, and the picture would show nowhere until the next change.

### Considered options

- The picture in the card, or a hash of it (rejected: the picture does not fit a QR, and with no server a hash points nowhere; the full record arrives with the person's first Sent Item or Shared Area).
- The card as JSON with its signature inside, `{"card":…,"sig":…}` like a Sent Item (rejected: the payload escaped inside a string, itself holding the record as an escaped string, makes the link longer; base64url of the bytes signed keeps them exact).
- Compressing the payload (rejected: the two cores' deflate write other bytes, and the keys and signatures, most of the card, do not compress).
- A short code in the card (rejected: it would be a claim nobody checks; it is computed from the id and the first key).
- A local mark on a person taken from a card, as `stripped` marks a blocked one (rejected: it would not travel when the Profile goes on to one's own Devices; the rule on pictures does).

### Consequences

- A person added from a card shows the placeholder until their full Profile arrives with a Sent Item or a Shared Area.
- Two of one's own Devices that changed the Profile apart at the same version, one the picture and one the name, now both keep the one with the picture.
- Whoever holds a card can pass it on; it says only who the person is, and the receiver sees them as not verified.
- Text after a link that starts with a letter or a digit, on the same line or the next, joins the card and makes it unreadable: blanks are left out because a mail program may break a long link, so they cannot mark where the card ends. The apps put the link last in what they share.
- A Device whose key is not bound to its Profile yet, one that took a Profile over before the binding reached it, makes no card; the next sync with another of the user's Devices binds it.
- The key a card pins goes out with every other pinned key in LAN sessions, as keys always have (ADR 0002): a person of a Shared Area who knows that Device's key can tell the user added its owner.
- Each further Device in the binding adds about 270 characters to the link.

## Profile card files (#144)

A link opens the app only once the site publishes `assetlinks.json` for every signing key, so a card link tapped in a messenger opened the docs page instead. "Send my Profile" therefore sends a small file, `Ann.obfcard`, which the phone and the desktop open as they open a Sent Item; the QR stays as it is, and nothing sent involves the website.

The file is one JSON object, keys sorted and no spaces, the same bytes from both cores: `{"card":"<payload>.<signature>","type":"oldbrainfix-card"}`. `card` is a card as above, made the same way but for one thing: its record is the own record as kept, picture and all, still signed again by the Device that makes the card, so the person shows with their picture at once. Its signature covers the payload as before; nothing outside `card` is signed, and nothing outside it is read but `type` and `format`. `type` tells a card file from a Sent Item or a bundle, which apps that sniff a file opened from elsewhere need, since a Sent Item is JSON too (`looksLikeProfileCard`, `looks_like_profile_card`). `format` follows ADR 0019 and 0034: absent or the whole number 1 is this version's, and 1 is never written; anything else, `2`, `"1"`, `1.0` or `null`, is a later version, and the file says "made by a newer version" (`CardTaken.needsUpdate`, `CardTaken::NeedsUpdate`) instead of being read. A file larger than 1 MiB, one that is no JSON object, has another or no `type`, or holds a card that is no `<payload>.<signature>` or does not check out, is no card (`unreadable`); the link prefix is not accepted inside a file. A file that reads goes the way a link goes, unchanged: the same checks, the same refusals of one's own card, a blocked person's and a copied id, the same "Send yours back" and `back`. A file is never in person, so the person stays not verified until the codes are compared.

The file is named after the own Profile's name (`profileCardFileName`, `file_name`): sanitised as a name is, each of `/ \ : * ? " < > |` made `_`, blanks and full stops taken off both ends, `_` put before a name Windows keeps for a device (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`, `LPT1`–`LPT9`, before the first full stop, in any case), and `Profile` for a name with nothing left; the ending is `.obfcard`, shared as `application/vnd.oldbrainfix.card` beside the Sent Item's `application/vnd.oldbrainfix.item` and the bundle's `application/vnd.oldbrainfix.sync`.

### Considered options

- The picture left out of the file as of the QR (rejected: a file has room, and without it the person shows the placeholder until their first Sent Item or Shared Area).
- The own signed record passed on as it is, with the signature of whichever own Device made it (rejected: the card re-signs its record whichever form it takes, and the bytes of the record are the same either way, so the full Profile still replaces nothing).
- The file as the link's text (rejected: a picture makes the card far larger than the 16 KiB a pasted text is read up to, and a text file says nothing of what it is, so a wrong one could not be told apart).
- The payload as JSON inside the file rather than as base64url (rejected: the signature would cover bytes the two cores' JSON writers would have to reproduce exactly; the card as sent keeps them exact, and the QR and the file share one reader).
- A new outcome "not a Profile card" beside `unreadable` (rejected: the apps already say "This is not a Profile card, or it was changed on the way" for it).
- The person's id or the date in the file name, as a Sent Item's says only the date (rejected: the receiver should see whose card it is before opening it; the name is what the card shows anyway).

### Consequences

- A card file is about 1 KB and, with a picture, about twice the picture's JPEG besides (base64 inside the record, base64url around the payload), a few tens of kilobytes for a photo; far under the 1 MiB read.
- Opening the file after the QR's card brings the picture (a record with a picture replaces one without at the same version); the QR's card after the file changes nothing.
- Cards sent as links before, and links pasted, still open: the reader of a card's text is unchanged.
- A QR read by the camera app of the phone rather than by Old Brain Fix leads to the address in the link, which shows nothing once the page is gone; the app's own scanner reads only what follows the `#`.
- A crafted file holding a lone surrogate or nesting deeper than serde_json reads may be "made by a newer version" in the Dart core and unreadable in the Rust core; both refuse it.

## Areas shared from afar (#108)

Amended by ADR 0023: a Profiles file in the shared folder or a bundle is `{"keys":[…],"profiles":[…]}`, the key records of the Devices the Area's grants name beside the Profiles, read before the entries. An offered Area keeps the Profiles it carries only at Accept, through the same path as a Shared Area's; Decline keeps none.
