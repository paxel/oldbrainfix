# Area keys, and the shared folder sealed in segments per Device and key

Every Area gets a random 256-bit key of its own, and everything outside an Area (Unsorted Notes, Routines of Unsorted Notes, Named Places) goes under one personal key of the Device group; Shared Areas (#37) will hand out single Area keys and never the personal one. Keys are local settings, handed over only inside an authenticated LAN session (Pairing or a paired sync, ADR 0003) and, for Devices that meet only through the shared folder, in a keys file sealed with the personal key; never in plain text. A scope may hold several keys: two Devices that made their own before they met keep both, a reader tries each, a writer seals with the one of the lowest id, and no key is ever replaced or dropped, so nothing written earlier becomes unreadable. Key rotation is left out for now.

The shared folder keeps Catlog's segmented layout (Catlog's ADR 0010), split by key: `<device>.<key id>.<n>.seg` holds one entry per line, sealed with XChaCha20-Poly1305 (24-byte random nonces; `cryptography` in Dart, RustCrypto's `chacha20poly1305` on the desktop) under the key of the entity's Area at the time it is written to the folder; a Note moved to another Area writes on under the new key, and a Device holding both reads both. A key id is the first 16 hex digits of the key's SHA-256 and says nothing about the Area. `<device>.manifest` stays plain: the format, a generation, the writer's version vector and its segments with key ids and line counts. `keys/<device>.<key id>.keys` carries the writer's Area keys and the signing keys it holds, sealed once with each personal key it holds; since ADR 0016 and 0017 also its own backup key and the own Profile, signed. Blobs lie in `blobs/` sealed with their Note's key, named by an HMAC-SHA256 of `photo/<hash>` under that key, so nobody holding a photo can tell it is there. What comes in is checked and applied exactly as a LAN import: signatures, Conflicts from causal contexts, a Moment before it.

A reader takes only the segments of keys it holds and remembers how far it read each; a segment of a key it lacks is counted as unreadable and read once the key arrives. Entries from the folder are never filtered by the version vector, because a segment read late fills a gap below it; only the store's "already held" check decides.

## Considered Options

- One run of segments per Device with the key id on every line (rejected: a reader lacking a key skips lines it can never come back to without reading everything again).
- Area keys derived from the personal key (rejected: one leaked personal key would open every Area, and a single Area could not be given a key of its own to share).
- Sealing the manifests too (rejected: a Member of a Shared Area must find the segments of the one key it holds).

## Consequences

- The folder shows who syncs and how much: device ids, version vectors, how many keys, segments and lines per key, and blob sizes. Nothing it holds tells what was written, in which Area, or by whom.
- Devices paired before keys existed, or that both made a personal key before they met, need one LAN sync before the folder alone carries everything.
- An entry relayed from a third Device below the writer's published vector is not written to the folder again; that Device's own segments carry it.
- Moving a Note into a Shared Area (#37) leaves its earlier entries under the old key; sharing will have to write the Note's history again under the Area's key.
- LAN deltas by version vector assume a Device holds every entry below its vector; a Device that cannot read some Areas breaks that, which Shared Areas (#37) have to settle.

## Versions (ADR 0019)

Amended by ADR 0019 (#59): the manifest writes its `format` only above 1, and a missing one reads as 1. A manifest of a newer format is announced as needing an update, not skipped silently, and nothing of its writer is read.
