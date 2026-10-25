# Devices of different versions sync where they can, pass on what they do not know and say which one needs an update

Devices running different versions of the app keep syncing as far as they can, and tell the user when one of them needs an update, instead of refusing silently or dropping data (#59). One rule holds for every version marker: a missing version means 1, and version 1 is never written. It holds for the text marker's `v` (ADR 0018), the Sent Item's `v` and `format`, the Backup's `format` in its header and its first record, the shared folder's manifest `format`, an entry's `v`, and the LAN wire format, whose body says `format` and whose header states a range. Reading stays tolerant: a `format` or `v` of 1 written out reads as 1.

## LAN and automatic sync

Each side states the oldest and the newest wire format it speaks in `x-oldbrainfix-format`, as `<oldest>-<newest>`, or one number for one format; a Device stating none speaks format 1. The joiner sends it with every request; the host answers `/vector`, `/sync` and its refusals with its own. Both take the newest format both speak, and the joiner's `/sync` body names it as its `format`, which the answer repeats. This release speaks format 4 only (`oldestSyncFormat`, `newestSyncFormat`).

When the two ranges share no format, nothing is sent. The host refuses `/vector` with `403 {"refusal":"format","formats":"<oldest>-<newest>"}` and its range in the header; a `/sync` body in a format it does not speak is refused the same way. The Device whose newest format is older than the other's oldest needs an update: the joiner fails with `SyncFailure.updateHere` or `updateThere`, and the phone's Sync page says which Device needs it.

An automatic sync, the phone's in the background included, tries every minute. So that a mismatch is not announced at every try nor swallowed forever, each side keeps it as an update notice, a local setting by the Device synced with (`update:<device>`, `here` or `there`), and announces it once at the top: the phone when it happens or when the app shows next, the desktop when its window reads the log next. A sync with that Device that goes through forgets the notice; a different side needing the update is news again. A joiner that did not name itself, one joining by pair code, is kept under `lan` and announced at every refusal: such a join is started by hand, each time anew.

## Shared folder and bundles

A manifest of a format newer than this version reads is no longer skipped as if half-written. Nothing of its writer is read, it is counted in the folder sync's result and the folder status (`newer`), and an update notice `folder` (`here`) is kept and announced once: another Device writes a newer format, this app needs an update. It is forgotten once no manifest in the folder is newer. A bundle holding only manifests of a newer format fails with `BundleNeedsUpdate` instead of `NotABundle`, and says the same.

## Entries

An entry may carry `v`, the version of its structure, omitted for 1; this version applies version 1 (`entryVersion`). A later version adds keys of its own after the keys of version 1. An added key a Device of an older version may ignore keeps `v`; keys that change what an entry means come with a higher `v`.

Canonical form: the keys of version 1 in their order (`device`, `dseq`, `entity`, `field`, `value`, `date`, `author`, `recorded`, `ctx`, `sig`), then every other key sorted, the keys of objects inside sorted too, no spaces (`canonicalEntry`). Both cores write entries so, and a later version keeps to it. Its added values keep to strings, whole numbers, booleans, null, lists and objects: fractions are not written the same by both cores.

Signature: an entry without other keys signs the same bytes as before (ADR 0002, 0011). An entry with other keys, `v` among them, signs the list `[device, dseq, entity, field, value, date, author, recorded, ctx or null, {the other keys, sorted}]`. A Device that knows none of those keys still checks the signature over all of them, so a key changed, added or dropped on the way is refused like any forged entry. The keys of version 1 keep their types in every version, since the signature is checked over them.

Every Device keeps and passes on an entry as it came. The store keeps the entry's JSON (`raw`) whenever it carries other keys or is not what this version writes for it, and its `v` when it is not 1. Segments, bundles and Backups carry the line itself; a LAN body carries it as a JSON object, read back in its canonical form, so for every Device that writes the canonical form the bytes stay the same, and the signature holds in every case. An entry with a `v` higher than this version applies is kept, counts for what is held (the version vector, the deltas, `holds`) and goes on to every other Device, but is not applied: no current value, no Conflict, no role. A `v` that is no whole number from 1 makes the line no entry.

## Considered Options

- An exact wire format, as before (rejected: two Devices one version apart could not sync at all, and only the joiner learned why).
- Skipping a newer manifest silently (rejected: the user never learns that a Device writes what this one cannot read).
- Unknown entry keys left unsigned (rejected: any Device on the way could add or change them unseen).
- Keeping only the keys this version knows (rejected: an older Device in between would strip what a newer one wrote, and its signature would break).
- Entries of a later structure dropped instead of kept (rejected: an older Device in between would hold back what the others need, and its version vector would ask for them forever).
- A second table for entries not applied (rejected: one table with `raw` and `v` keeps one notion of what is held).

## Consequences

- Manifests, Backup headers and Sent Items no longer write `"format":1`; the fixtures changed.
- The store gained the columns `raw` and `v`; a log from before gains them empty.
- Reading what applies filters on `v`; what is held and relayed does not.
- Nothing changes for users of this version alone: an announcement shows only once a Device of another version speaks to it.
