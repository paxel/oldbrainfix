# Tags are words of the text, found by the search

A `#word` in a text, such as "#Garten", is drawn highlighted, and a tap or a click on it opens the search for it (#126). Both cores find tags the same way (`text_tags.dart`, `text_tags.rs`). A tag here is only such a word of the text; it is no Topic and no part of the Capture Context, which the app never calls tags.

## A tag is part of the text

Nothing is stored for a tag; the tag tree (#267, below) is read from the texts. A tag is signed, synced, judged by the roles of a Shared Area (ADR 0013), won by latest wins and made a Conflict with the rest of the text (ADR 0011), and a text with tags reads the same on a Device that does not know them.

## What counts as a tag

A tag is a `#` (U+0023) followed by tag characters: letters, digits, `_` and `-`, and `/` inside since #267 (below). A letter is a character of Unicode's Alphabetic property, so umlauts, "ß" and every script count; a digit is a character of its number categories. Both cores take these properties from Unicode as their platform knows it; the characters people type agree, while a character added to Unicode lately may not.

- The tag runs to the first character that is no tag character, so "#garten." and "(#garten)" hold "#garten", and "#a#b" holds "#a".
- The first character after the `#` is no `-`, and any `-` at the end are left out, so "#Garten- und Balkonpflanzen" holds "#Garten". A `-` inside counts: "#to-do".
- At least one letter: "#1", "#2024" and "issue #126" are numbers and no tags, while "#v2" and "#2024plan" are tags.
- A `#` with a space after it, such as a heading "# Title", a lone `#`, and "##" are no tags.
- Before the `#` comes the start of the text or any character that is no tag character and none of `#`, `/`, `\` and `&`: a space of any kind, a line break, punctuation, an emoji or a status mark (ADR 0031). So "C#", "F#m", "page.html#anchor" and "example.org/#top" are no tags, nor is the Markdown escape "\#" or an HTML character reference such as "&#x27;".
- A combining mark is no letter, so a "ü" written as "u" and a combining diaeresis ends a tag; keyboards write "ü" as one character.

Offsets are each core's own, UTF-16 code units in the Dart core and UTF-8 bytes in the Rust core, and run from the `#` to the end of the tag. The tag as searched is the `#` and the tag's characters in lower case, each lowered on its own as the Dart core's `toLowerCase` does it, so "İ" becomes "i" and a closing "Σ" "σ" in both cores. The search lowers its query and the texts it searches the same way in both cores. No other normalisation is done. The fixture `text_tags.json` compares the tags as written, as searched and where they lie, counted in code points.

## Searching for a tag

The search stays as it is: a query is found in a text as typed, ignoring case, in the text of a Note and of its Sections, in Transcripts, Photo Texts, Tally entries and the Lines of a Checklist or a Shopping List. A tap on a tag opens the search with the tag as written. So "#Garten" finds every Note holding "#garten" in any case, and not one that only says "Garten". It also finds "#Gartenarbeit", which begins with it, and a "#garten" that is no tag, such as in "C#garten"; finding a little more is better for a memory aid than missing a Note, and typing "#gar" finds tags as they are typed.

## Formatted text (ADR 0033)

Amended by ADR 0033 (#127): in a formatted text, a `#word` inside inline code or inside a link's destination is no tag; one in a link's label, a heading's text or bold is. Search looks in a formatted text's visible text, without its marks.

## Nested tags and the tag tree

Amended by #267 (Q623, Q639): `/` is a tag character, so "#life/health/back" is one tag, nested in three levels. A Note can sit in several branches at once ("#health/back" and "#insurance/claims"), where a Topic holds it in one place, and Obsidian reads the same tags. A tag still needs no field, sync, role or conflict rule of its own, since it is part of the text.

- Each part between two `/` starts as the tag does: with a tag character that is no `-`. So "#/a" is no tag, a `/` at the end is left out as a `-` is ("#life/" holds "#life"), and `//` or `/-` ends the tag ("#a//b" and "#a/-b" hold "#a"). Inside a part a `-` counts as before; "#a-/b" is one tag.
- The tag still needs a letter somewhere: "#2024/05" is no tag, "#q1/2026" is one.
- A `#` after a `/` stays no tag, so "x/#a", "example.org/#top/a" and "#a/#b" hold no tag after the `/`. A `#` after a letter, as in "page.html#a/b", is none either.
- The tag as searched is lowered as before, the `/` kept. The search for "#life" finds "#life/health/back", since it searches by prefix, and the search for "#life/" its tags below.
- A label of another app still becomes a flat tag (`fromLabel`, `from_label`): its `/` becomes a `-` like any other character that is no letter, digit or `_`, so the tags an Import wrote stay the same.

The tag tree (`TagTree`, `tag_tree.rs`) lists every tag by its levels. It reads the texts of the Notes neither in the trash nor merged and of their Sections, the merged Notes' texts among them, with each text's tags as it draws them: none in a Code text, none in code, in a link's destination or in a web address (ADR 0033). Nothing is stored for it. Each level is one node: its name is the part as it is written most often, counted over every tag in the texts, the first by UTF-16 code units on a tie; its tag is `#` and the names down to it, which is what a tap or click on it searches for; its count is the number of Notes holding it or a tag below it, each Note once. The nodes of a level come in the order of their names in lower case, by UTF-16 code units. The suggestions for a search being typed are the nodes whose tag begins with what is typed, ignoring case, each before those below it, and none when it does not begin with `#`. The fixture `tag_tree.json` holds both, and `text_tags.json` the nested tags. The AI port reads the tree with `list_tags` (ADR 0026).

## Considered Options

- Searching for a tag as a whole tag, so that "#garten" does not find "#Gartenarbeit" (rejected: a query being typed would find nothing until the tag is complete, and a Note filed under a longer tag would be missed).
- Numbers as tags (rejected: "#126" and "Gleis #3" are numbers people write, not labels).
- Only `_` and no `-` in tags (rejected: German compounds and "to-do" are written with `-`).
- `//` inside a nested tag kept, as an empty level (rejected: a level without a name could not be shown or searched for; `//` is more often a web address or a typo).
- A label of another app with a `/` imported as a nested tag (rejected: Keep's labels are flat, and the tags of earlier Imports would differ from new ones).
- The tag tree from every text search reads, Transcripts and Photo Texts included (rejected: a "#" heard or photographed is no tag the user wrote, and no text draws it as one).

## Consequences

- Tags need no new field, no new format and no migration, and Sent Items, Bundles and Backups carry them as text.
- Tags show in every text the app draws: Note texts and Section texts, on cards and in the editor's view, on the phone and the desktop.
