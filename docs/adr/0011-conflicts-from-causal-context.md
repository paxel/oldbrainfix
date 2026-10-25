# Conflicts are computed from a causal context on each edit, not flagged at import

Unlike Catlog, which writes a `$conflict:<field>` entry when an import detects concurrent edits, every edit of a conflictable field (Note and Section text, Area, Topic and Named Place names, Named Place radius, a Routine's schedule as one unit) carries a signed causal context: the writer's version vector limited to the other Devices that wrote that field. A Conflict exists while the entries nobody has seen since hold different values, so it is derived from the log on every Device alike, nothing is written at import, and any later edit that saw both versions ends it everywhere. Entries without a context (first writes, and everything written before this) never start a Conflict. The context is signed as an optional ninth element, so older entries keep their signatures; the sync format went to version 2 so older builds refuse cleanly.

## Considered Options

- Catlog's `$conflict` marker entries (rejected: a Device that imports late writes markers of its own, and clearing them needs more entries).

## Text formats (ADR 0018)

Amended by ADR 0018 (#58): a Note's or Section's text carries its format in the same value, so text and format conflict as one unit. A text written before its format was recorded counts as the same text marked plain. A Conflict one of whose versions is in a format or a structure version (`v`) the Device does not know is not resolved there.
