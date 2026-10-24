# Copy and adapt Catlog's code instead of sharing a library

Old Brain Fix starts from copies of cat(a)log's pure-Dart core (store, entry log, sync, signing, ICS) and its Rust desktop crates, and from Chaos Alert's Kotlin alarm layer. The copies are free to drift apart from their origin. The domains differ (cats versus notes), Catlog is already mature, and a shared library would force every change to keep two apps working for little gain.

## Considered Options

- Extract shared packages and crates used by both apps (rejected: coupling between unrelated release cycles).
- Rewrite from scratch using Catlog only as a pattern (rejected: throws away working, tested sync code).

## Consequences

Fixes made in Catlog's sync or signing do not reach Old Brain Fix automatically; they have to be ported by hand when relevant.
