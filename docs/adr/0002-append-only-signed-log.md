# Append-only signed entry log with version vectors, latest wins

Like Catlog (its ADR 0001 and 0008), every change is an entry in a per-Device append-only log, signed with Ed25519, ordered by version vectors. The latest entry wins, and concurrent edits are flagged as conflicts for the user to resolve. We keep the model so Catlog's sync and its Dart-to-Rust fixture tests carry over nearly unchanged, and because signed entries are what lets Devices enforce Shared Area roles (Managers and Members) and Sent Item ownership (ADR 0009).

## Considered Options

- Plain rows with an updated-at timestamp, last writer wins, no signing (rejected: Catlog's sync could not be reused, and roles could not be enforced).
