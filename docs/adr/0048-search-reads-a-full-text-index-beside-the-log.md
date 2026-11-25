# Search reads a full-text index beside the log

Search read every Note on each query: their texts, Sections, Transcripts, Photo Texts, the names and Attachment Texts of their files, Tally lines and the Lines of Checklists and Shopping Lists, each lowered and searched for the query (`search.dart`, `search.rs`). With documents' texts (ADR 0047) and asking (#267) a store holds far more text, and at 20,000 Notes a query took 2.3 s in the Dart core and 1.3 s in the Rust core on the development machine. Both cores now keep a full-text index, SQLite FTS5, and find exactly the Notes they found before (#267).

## What the index holds

The index lies in the store's own database, beside the log: `search_texts`, an FTS5 table with the trigram tokenizer and `case_sensitive 1`, and `search_state` with the index's `version` and its mark `seq`. Each entry that wrote a text search reads is one row: its entity and its text folded letter by letter by the cores' own rule (see Folding below; it was lowered only up to `version` 1), never by SQLite's. The fields are `text`, `line`, `transcript:`, `ocr:`, `fileText:` and `file:`, of which the file's name counts (`Attachment.read`, `Attachment::read`). A `text` holds the ways search reads one, a line apart: as a Note's with its boxes as written, as a Section's and as a Line's (ADR 0033, ADR 0035).

The index narrows and never decides. It names every entity one of whose texts ever held the query, and search reads those as they are now: latest wins, what the roles of a Shared Area leave out (ADR 0013), the end of a Merge chain, the making of a Checklist or Shopping List, a gone or deleted Line, a trashed Tally entry, the trash and the Area or Topic searched in. An older value, an entry a role leaves out or one deleted when a person was blocked (ADR 0021) is no more than a candidate too many. An index of the values as they count now would have to decide each of these again on every write; one of every value written can find too much, which search reads away, and never too little.

## Kept current

Before each search the index takes in the entries after its mark, the log's high-water mark it last read (ADR 0002), in one transaction, or inside the batch the search runs in. So every write counts, whichever connection made it: the phone's background sync in its isolate (#225), the desktop's sync on its thread, or an older version of the app that does not know the index. The index is not written by the writes themselves, so a write costs what it did; it takes the database's write lock as a batch does and reads the mark again under it, so two connections never take in the same entries twice. While another connection holds the lock, a search waits for it up to the store's busy timeout of 5 s and then reads every Note. It is built anew from the whole log when a table or the mark is missing, when the version is another or when the mark lies beyond the log. The changes of the shown Pending Changes (ADR 0022) are not in the log, so their entities are read as candidates on each search; a thread that reads the log alone (ADR 0026) reads them as the log has them.

The database's `user_version` stays 1 (ADR 0034): the tables only repeat what the log holds, an older version leaves them alone, and the mark makes this one take in what an older one wrote.

## A query

A query is trimmed and folded, and a blank one finds every Note in scope; one that is not blank but folds to nothing, such as an accent alone, finds none. One of three characters, counted folded, or more is looked up as an FTS5 phrase, its `"` doubled, and the rows are checked to hold it as typed (`instr`), spaces and marks included. Trigrams find nothing shorter, so a query of one or two characters is looked for in every row's text, which SQLite does without reading a Note. A query holding a box's glyph, ☐, ✅ or ❌, reads every Note as before: a tick changes the glyph a box shows, and the rows hold the one it was written with.

The Notes found come newest capture first, and those captured at the same moment in the order the store lists the Notes in (`everOfType`, `ever_of_type`), as the Rust core's stable sort of every Note keeps them. `Search.among`/`Search::among` keep the order of the list given, as before.

## Folding (obf-1047)

A hit ignores case, accents and umlauts, and nothing more: "muller" finds "Müller", "strasse" finds "Straße", "mueller" does not find "Müller", and no typo is allowed for. Search, the index and the places of a query in a Note match by the same folding (`Findings.fold`, `findings::fold`), so what the list finds and what the find bar shows agree. Each character folds on its own, so a text folds as its characters do:

1. In lower case, by a table both cores carry (`_lowerRuns`, `LOWER_RUNS`, Unicode 18 as Rust 1.100 has it), the first character where the lower case is more than one ("İ" to "i"). The platforms' own lower case is not used: Dart 3.14 lacked 485 pairs Rust's has (Cherokee, Georgian Mtavruli, Osage, Adlam and newer Latin letters among them), and either changes with an update, which would change what a core finds without a change to its code.
2. A combining accent, U+0300 to U+036F, is dropped, so a decomposed "u" with its diaeresis folds as "ü" does.
3. "ß" and "ẞ" fold to "ss", "æ" to "ae" and "œ" to "oe".
4. A Latin or Greek letter whose canonical decomposition is a letter and combining accents folds to that letter, and so do the letters with a stroke ø, đ, ħ, ł and ŧ and the dotless ı (`_foldRuns`, `FOLD_RUNS`). Cyrillic letters are only lowered: "й" is a letter of its own, not "и" with an accent.

The index's `version` is 2 since then, so an index of lowered texts is built anew at the first search after the update. `desktop/fixtures/findings.json` lists what every character that does not fold to itself folds to, and the Rust core checks every character against it.

## Where a query is found (obf-1047)

`Findings.inText`/`findings::in_text` give the places of a query in a text as it shows: a Note's or Section's shown text (its boxes as glyphs, ADR 0035), a Transcript, a Photo Text, an Attachment Text or a file name. A formatted text is matched line by line as it reads without its marks, as search reads it: a table's row as its cells that show something, a space between them; a table's delimiter row, the line marking its first column and a code block's fence are no lines. Each place runs from the first character shown to the last, marks inside it included, in the shown text's offsets (UTF-16 code units in the Dart core, bytes in the Rust core), with its line, for a table's row the table (by its index and its start), the row (the header 0) and the column it begins in, and its line as a context, cut around the place at whole words when longer than 80 characters. A match on half of a character's folding takes the character, and two matches on the same character are one place. `Findings.ofNote`/`findings::of_note` give those of a Note: its text, Transcripts, Photo Texts, file names and Attachment Texts, then the same of each Section given. A Note found through a Tally entry's line or a Line of its Checklist or Shopping List has no places, and a query holding a line break none; search still finds those.

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
- The first search after the update builds the index; later ones take in only what was written since. The update that folds builds it once more (obf-1047).
- What search reads stays decided in one place, the code that read every Note, which a test of each core compares the index with.
