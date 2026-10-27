# Append-only signed entry log with version vectors, latest wins

Like Catlog (its ADR 0001 and 0008), every change is an entry in a per-Device append-only log, signed with Ed25519, ordered by version vectors. The latest entry wins, and concurrent edits are flagged as conflicts for the user to resolve. We keep the model so Catlog's sync and its Dart-to-Rust fixture tests carry over nearly unchanged, and because signed entries are what lets Devices enforce Shared Area roles (Managers and Members) and Sent Item ownership (ADR 0009).

## Considered Options

- Plain rows with an updated-at timestamp, last writer wins, no signing (rejected: Catlog's sync could not be reused, and roles could not be enforced).

## Blocks (ADR 0021)

Amended by ADR 0021 (#90): the entries a blocked person's Devices wrote are deleted from the log, but their check-offs, grants and the filing of their Notes; it is the only deletion there is. The version vector a Device tells others counts what it refused.

## Pending Changes (ADR 0022)

Amended by ADR 0022 (#111): changes the user has not accepted yet wait in a pending file beside the log, where the reads see them. This Device numbers and signs what it writes only when the change reaches the log at Accept, so its sequence never has holes. Entries other Devices signed wait there as they came and are taken in at Accept as any import is. Decline deletes the file, and the log never saw anything.
