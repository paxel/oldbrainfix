# Shared Areas: roles per Device signed into the Area, enforced at projection, sent only in scope

A person shares one Area with another by Pairing in person, in a mode that invites the other Device to that Area as Manager or Member. Either side may be the inviter: a host shows an invite code (its `/vector` answer says `x-oldbrainfix-invite`, so the joiner sends nothing of its own), or a joiner scanning a plain code says `invite` in its `POST /sync`. The sync format went to version 3 so older builds, which would answer an invite with everything, refuse cleanly.

Roles are per Device key, signed into the Area as `role:<device>` on the Area entity, so they travel under the Area key with no plumbing of their own. The value is `{"name":…,"person":…,"role":"manager"|"member"}`; null removes. The person groups a person's Devices: the inviter types the name, the person id is the invited Device's id, and a Device with a role gives a Device of its own group (paired without an invite) the same role, person and name when they sync and it has none. The first Device to share an Area grants itself Manager (the first Manager), and its own paired Devices with it. A grant counts when a Manager signed it, or when it is that first self-grant, or when a Member gave Member under its own person to a Device that had no grant yet.

Every Device works out from the raw log which entries to ignore, the same way in both cores (`shared_areas.dart`, `shared.rs`), and every projection (`history`, the current fields, the entity lists) leaves them out; nothing is refused at import, so a Device that reads an entry it may not act on still relays it. The order is fixed: grants by latest wins; then Topics by their Area; then Notes by their Area or Topic; then Sections and Routines by their Note, each entry judged in the Area its entity was in at that point (its first filing for entries before it). An entry dated before the Area's first grant counts as the owner's. Otherwise a Device's role at an entry is its last grant before it, or its first grant when it had none yet (so a Device granted late keeps what it wrote meanwhile). Managers may do everything; Members may write Notes they own (a Note belongs to the person of the Device that wrote its first `$type`) and Sections they own (so they Amend any Note), check off and uncheck Occurrences (`done@…`), and add Transcripts and Photo Texts to any Note; nothing else of Routines, Topics or the Area. A Device without a role, or removed, is ignored in a Shared Area.

What leaves a Device for a Device paired by invite is cut to the Areas that Device holds a role in now: entries whose entity lies in one of them, those Areas' keys and never the personal one, the blobs of those Notes, and `wantBlobs` and blob requests likewise. Removing a person ends its roles in the log, so every Device of the group stops sending to it. A Device paired by invite stays one, even when it later pairs with a plain code. Keys a Device paired by invite hands over are taken only for the Areas shared with it. The invite session sends the Area's whole history, not a delta; afterwards a delta by vector, to which the whole history is added of every Note filed into a Shared Area within the delta (with its Sections and Routines), since a Note captured Unsorted and filed later has its start below the vector. The shared folder does the same: it adds the same histories, sealed under the Area's key.

A Shared Area reuses whatever shared folder both people reach, with no folder of its own. Segments are already per key (ADR 0012), so the other person reads the Area's segments and nothing else. An Area with a Starter Set id is moved to fresh ids (Area and Topics, the Notes filed again) before it is shared, since every Device plants those ids and the other person's private Household would otherwise be the same Area. An invited Device refuses an Area it already holds and nobody shared.

## Considered Options

- A person key that signs its Device keys (rejected for now: a key and a ceremony of its own; the person id inside the grant groups Devices well enough for Note ownership and for removing a person at once).
- Roles at their current value for every entry (rejected: demoting or removing someone would make their past Notes disappear).
- Refusing disallowed entries at import (rejected: a Device that cannot judge an entry yet, lacking the grant that comes later, would drop it for good).
- Sending the whole Area on every sync (rejected: a year of check-offs every minute).
- A shared folder per Shared Area (rejected: a second folder to pick and keep, for what the per-key segments already give).

## Consequences

- Removing someone only stops sending: they keep the key and what they had, and a shared folder still carries what is written after. Real revocation needs key rotation, which is still left out (ADR 0012).
- Roles keep honest people from mistakes; they do not stop a modified app. Dates are the writer's word, so a backdated entry counts as the owner's, a Device key made up by someone holding the Area key can be given Member by them, and entries a person's Device writes outside its Areas are not refused by the Devices it reaches.
- A Device of the other person joins through Pairing with them, or a Device of its own that already has a role; a Device that meets them only through the folder needs one LAN sync first to learn the signing keys.
- A Note moved out of a Shared Area stays as it was for the others.
- Phones only join, so two people with only phones cannot share yet.

## Profiles (ADR 0017)

Amended by ADR 0017: the name a Manager types for the invited person goes; the invite shows the other person's Profile to confirm, and the person of a Note is its Profile, found through the signing Device's binding and the aliases. Profiles of the people of a Shared Area travel with its sync and bundles (#50).
