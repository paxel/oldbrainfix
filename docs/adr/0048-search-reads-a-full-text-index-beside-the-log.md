# Search reads a full-text index beside the log

Search read every Note on each query: their texts, Sections, Transcripts, Photo Texts, the names and Attachment Texts of their files, Tally lines and the Lines of Checklists and Shopping Lists, each lowered and searched for the query (`search.dart`, `search.rs`). With documents' texts (ADR 0047) and asking (#267) a store holds far more text, and at 20,000 Notes a query took 2.3 s in the Dart core and 1.3 s in the Rust core on the development machine. Both cores now keep a full-text index, SQLite FTS5, and find exactly the Notes they found before (#267).

## What the index holds

The index lies in the store's own database, beside the log: `search_texts`, an FTS5 table with the trigram tokenizer and `case_sensitive 1`, and `search_state` with the index's `version` and its mark `seq`. Each entry that wrote a text search reads is one row: its entity and its text in lower case, lowered letter by letter by the cores' own rule (ADR 0032: "İ" to "i", a closing "Σ" to "σ"), never by SQLite's. The fields are `text`, `line`, `transcript:`, `ocr:`, `fileText:` and `file:`, of which the file's name counts (`Attachment.read`, `Attachment::read`). A `text` holds the ways search reads one, a line apart: as a Note's with its boxes as written, as a Section's and as a Line's (ADR 0033, ADR 0035).

The index narrows and never decides. It names every entity one of whose texts ever held the query, and search reads those as they are now: latest wins, what the roles of a Shared Area leave out (ADR 0013), the end of a Merge chain, the making of a Checklist or Shopping List, a gone or deleted Line, a trashed Tally entry, the trash and the Area or Topic searched in. An older value, an entry a role leaves out or one deleted when a person was blocked (ADR 0021) is no more than a candidate too many. An index of the values as they count now would have to decide each of these again on every write; one of every value written can find too much, which search reads away, and never too little.

## Kept current

Before each search the index takes in the entries after its mark, the log's high-water mark it last read (ADR 0002), in one transaction, or inside the batch the search runs in. So every write counts, whichever connection made it: the phone's background sync in its isolate (#225), the desktop's sync on its thread, or an older version of the app that does not know the index. The index is not written by the writes themselves, so a write costs what it did; it takes the database's write lock as a batch does and reads the mark again under it, so two connections never take in the same entries twice. While another connection holds the lock, a search waits for it up to the store's busy timeout of 5 s and then reads every Note. It is built anew from the whole log when a table or the mark is missing, when the version is another or when the mark lies beyond the log. The changes of the shown Pending Changes (ADR 0022) are not in the log, so their entities are read as candidates on each search; a thread that reads the log alone (ADR 0026) reads them as the log has them.

The database's `user_version` stays 1 (ADR 0034): the tables only repeat what the log holds, an older version leaves them alone, and the mark makes this one take in what an older one wrote.

## A query

A query is trimmed and lowered as before, and a blank one finds every Note in scope. One of three characters or more is looked up as an FTS5 phrase, its `"` doubled, and the rows are checked to hold it as typed (`instr`), spaces and marks included. Trigrams find nothing shorter, so a query of one or two characters is looked for in every row's text, which SQLite does without reading a Note. A query holding a box's glyph, ☐, ✅ or ❌, reads every Note as before: a tick changes the glyph a box shows, and the rows hold the one it was written with.

The Notes found come newest capture first, and those captured at the same moment in the order the store lists the Notes in (`everOfType`, `ever_of_type`), as the Rust core's stable sort of every Note keeps them. `Search.among`/`Search::among` keep the order of the list given, as before.

## Where FTS5 is

Both cores' SQLite has FTS5 with the trigram tokenizer: the Dart core's `package:sqlite3` bundles a SQLite compiled with `SQLITE_ENABLE_FTS5` on every platform, the phones included, and the desktop's `rusqlite` builds its bundled SQLite with it. Where the index cannot be made or read all the same, search reads every Note as before, so nothing is missed.

## Measured

20,000 Notes of 8 to 37 words, every tenth with an Amend (68,000 entries), in a store in memory, each query once, on the development machine; the Dart core on the Dart VM, the Rust core in a release build.

| | Dart core | Rust core |
| --- | --- | --- |
| Every Note read (before), any query | 2.25–2.29 s | 1.29–1.30 s |
| Building the index, first search | 0.25 s | 0.20 s |
| A query no Note holds | under 1 ms | under 1 ms |
| "dentist bring", 488–495 Notes | 30 ms | 32 ms |
| "sauce", about 10,500 Notes | 0.58 s | 0.37 s |
| "e", every Note | 1.11 s | 0.70 s |
| A Capture, then a search | 18 ms, then under 1 ms | 18 ms, then under 1 ms |

A query found in most Notes still reads each of them, which is what listing every Note costs (1.06 s and 0.60 s).

## Considered Options

- FTS5's default tokenizer, by words (rejected: "milk" would not find "oatmilk" nor "#garten" "#gartenarbeit", and its case folding is not the cores'; search finds what is typed, anywhere).
- A row per Note with its texts as they count now (rejected: what counts follows from latest wins, roles, Merges and a list's making, and from other Notes; a row kept current on every write could go stale where the scan never did, and a query could match across two texts).
- An index in memory, built when the app opens (rejected: the cost at every start, and the phone's background sync writes on another connection).
- A plain table searched with `instr` alone (rejected: it reads every row on every query; FTS5 reads the trigrams of the query).

## Consequences

- The database grows by the searchable texts and their trigrams.
- The first search after the update builds the index; later ones take in only what was written since.
- What search reads stays decided in one place, the code that read every Note, which a test of each core compares the index with.
