# Sharing an Area from afar: an offer bundle sealed to the person's keys, held as Pending Changes

A user can share an Area with a person in People who is not on the same network (#108): one added from a Profile card (ADR 0017, #107), or met before. Pairing by invite (ADR 0013) needs both Devices on one network, so the grant, the Area key and the Area's history have to travel in a file instead, through any messenger, and later changes by the shared folder or by bundles. Both cores do it the same way (`area_offers.dart`, `offers.rs`).

## Sharing

"Share with…" shares the Area as before (ADR 0013): a Starter Set Area moves to fresh ids, and this Device becomes its first Manager. It then grants the person the role chosen on every Device of theirs an offer can be sealed for: pinned here, bound to their Profile, with an X25519 key (ADR 0015), never one of the user's own. A person none of whose Devices is known so is not offered. The grants are written at once, as an invite writes them, and the user is asked once how changes travel with this person: by a shared folder both reach, or by bundles. The answer is a local setting of this Device, `transport:<profile id>` = `folder` or `bundle` (ADR 0008).

## The offer

The offer is a bundle (ADR 0003) of the Area's whole history: every entry whose entity lies in the Area now, with the photos and recordings it names and the Profiles of the Area's people, sealed with the Area's key as any bundle is (ADR 0012). Beside the segments lies a file `offer`, one JSON object, keys sorted and no spaces, the same bytes from both cores: `{"sealed":…,"to":[{"eph":…,"key":…}]}`. It is sealed as a Sent Item is (ADR 0015): a random content key seals `{"offer":<payload>,"sig":<Ed25519 over "oldbrainfix:area-offer:" + payload>}` with the associated data `oldbrainfix:area-offer`, and each stanza seals the content key for one Device of the person, through HKDF-SHA256 with the info `oldbrainfix:area-offer-key`. The prefix, the associated data and the info are the offer's own, so neither a Sent Item nor an offer stands in for the other. The payload is `{"area":…,"areaKeys":[…],"from":<key record>,"keys":[…],"profile":<signed Profile>,"transport":…}`: the Area's id and its keys, the key record of the Device that signed, the key records of every Device that wrote in the Area or that its grants name, held here, the own Profile as it goes to other people, and how changes travel. An offer with a key of another scope, or with none, is no offer.

The same offer is written for both ways of travelling: the shared folder carries later changes, but not the Area's key, nor the entries of Notes filed into the Area before it was shared, which lie under the personal key (ADR 0012).

## Receiving

A bundle that holds `offer` is read as an offer, not imported. The offer is opened with this Device's X25519 key, its signature checked against the key record it names, and the Profile it carries must bind that key, as a Sent Item's must. Then, before anything else is read: an offer from a blocked person is refused (ADR 0021); one of an Area this Device holds as its own and nobody shared is refused, as an invite to it is (ADR 0013); one this Device holds a role in already is imported as any bundle.

The entries are read from the bundle's segments with the offered key, which is not kept. Only entries whose entity lies in the offered Area by what the entries themselves say are taken: the Area, its Topics, the Notes filed into either, their Sections and Routines; and of these none whose entity this Device holds already in another scope, so an offer never moves the user's own Notes or places into somebody else's Area. The role is read from the grants, judged over the log and these entries together, never from the payload: this Device must hold a grant there under the user's own Profile, and the Device that signed must hold Manager under the Profile the offer carries; otherwise the offer is not for this Device. A Member who passes an Area on does not offer it.

The offer is held as Pending Changes of kind `area-offer` (ADR 0022), never shown: the entries as received, checked as any pending entry is, the key records in the set's `keys`, and as `payload` `{"blobs":[…],"offer":<payload>,"profiles":[…]}`, the offer as it came, the Profiles read with the offered key, and the photos and recordings kept for it. Those are put into the blob store when the offer comes, only those this Device lacked, since a pending set keeps no bytes (ADR 0022); the log does not refer to them, so nothing shows or sends them. The same offer received again is the one held. The preview shows the sharer's Profile with its short code, as verified, known or new, as a Sent Item's does; the Area's name and Starter Set slug as the entries say, leaving out what the roles leave out, so a Member's rename relayed with the history names nothing; and the role: "Ann shares Household with you as Member". An offer whose grants hold only by entries whose signatures fail is declined at once.

## Accept and Decline

Accept checks the sharer against the blocks and the Area against this Device's own again, keeps the Area's key, for that Area only, and then accepts the set: the history is taken in as any import is, its keys pinned (ADR 0022). The key comes first, so a stop in between never leaves a history that cannot be read on. Then the Profiles the Area's grants name are taken in as a Shared Area's are (ADR 0017), the sharer's verified when the user compared codes, and how changes travel with the sharer is kept as the offer says. A Moment comes before what was taken in. Decline deletes the set and the photos and recordings kept for it that the log does not refer to: no entry, key, Profile or setting stays. "Decline and block" declines it and blocks the sharer as a Sent Item's sender is blocked (ADR 0021), keeping their Profile without its picture so the "Blocked" list names them. A block declines every offer held from that person.

## Afterwards

The Devices of a person met from afar are another person's Devices: a bundle for one of them, or for the person, holds only the Areas shared with them, with the whole history of Notes filed into them meanwhile (ADR 0013). Another person's Device is one paired by invite, one a grant names for another person, or one bound to the Profile of another person kept here. A bundle for a person goes to all of their Devices at once: what lies beyond what any of them said it holds. The person's Devices sync with each other and pass it on.

Through the shared folder and in bundles, the Profiles file of a Shared Area, `keys/<device>.<key id>.profiles`, sealed with the Area's key, also carries `keys`: the key records of this Device and of every Device the Area's grants ever named, held here. A reader takes them in before the entries, from every Profiles file sealed with a key of an Area it holds. A key record names the Device its key belongs to, so a record from anyone holding the Area key pins nothing that is not that Device's. So a person's second Device, given its role by their first one (ADR 0013), is checked by the others without having met them.

## Considered Options

- The offer in the shared folder, found there by the other person's folder sync (rejected: a declined offer would come back with every sync unless the decline were remembered, and an offer can only be sent once a folder is set up on both sides; a file through a messenger reaches the person either way).
- The history inside the sealed offer, without a bundle around it (rejected: the photos and recordings would not come with it, and the bundle's segments, sealed with the Area key, already carry the history the same way every bundle does).
- The role in the payload (rejected: it would be a claim the grants do not back; the grants are signed entries and decide everything else in the Area).
- Showing the offered Area before Accept (rejected for now: a shown set pauses the sync while it waits, ADR 0022; the preview names the sharer, the Area and the role).
- Keeping the photos and recordings out until Accept (rejected: a pending set holds no bytes, and the bundle the offer came in may be gone by then).
- Cutting a bundle for every Device that is not one of the user's own, unknown ones too (rejected for now: bundles for one's own Devices that never paired directly would lose everything outside the Shared Areas; a Device known as another person's is cut).

## Consequences

- The grants are written when sharing: until the person accepts, and also when they decline, they are listed among the Area's people, and the user removes them as any other person.
- The offer's key records are only those held when it is written; a Device of the sharer's that joins later reaches the person through the Profiles file of the Area.
- Photos and recordings of an offer that waits lie in the blob store until it is decided.
- A person reading the shared folder sees the segments of everything the user writes there and counts those whose key it lacks as unreadable, as with any Shared Area (ADR 0012).
- A person who takes over another Profile (ADR 0017) after the offer was written finds it not for them; the user shares again.

## Changes to earlier decisions

- ADR 0012: the Profiles file of a Shared Area carries the key records of the Devices its grants name, read before the entries.
- ADR 0013: an Area is shared from afar by an offer file as well as by Pairing by invite; a bundle for another person's Device is cut to what it shares also when that Device was never paired.
- ADR 0017: a Profiles file has `keys` beside `profiles`.
- ADR 0022: an offered Area is a set of kind `area-offer`, never shown, whose photos and recordings are kept in the blob store until it is decided.
