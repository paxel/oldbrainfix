# Tags are words of the text, found by the search

A `#word` in a text, such as "#Garten", is drawn highlighted, and a tap or a click on it opens the search for it (#126). Both cores find tags the same way (`text_tags.dart`, `text_tags.rs`). A tag here is only such a word of the text; it is no Topic and no part of the Capture Context, which the app never calls tags.

## A tag is part of the text

Nothing is stored for a tag and there is no list of tags. A tag is signed, synced, judged by the roles of a Shared Area (ADR 0013), won by latest wins and made a Conflict with the rest of the text (ADR 0011), and a text with tags reads the same on a Device that does not know them.

## What counts as a tag

A tag is a `#` (U+0023) followed by tag characters: letters, digits, `_` and `-`. A letter is a character of Unicode's Alphabetic property, so umlauts, "ß" and every script count; a digit is a character of its number categories. Both cores take these properties from Unicode as their platform knows it; the characters people type agree, while a character added to Unicode lately may not.

- The tag runs to the first character that is no tag character, so "#garten." and "(#garten)" hold "#garten", and "#a#b" holds "#a".
- The first character after the `#` is no `-`, and any `-` at the end are left out, so "#Garten- und Balkonpflanzen" holds "#Garten". A `-` inside counts: "#to-do".
- At least one letter: "#1", "#2024" and "issue #126" are numbers and no tags, while "#v2" and "#2024plan" are tags.
- A `#` with a space after it, such as a heading "# Title", a lone `#`, and "##" are no tags.
- Before the `#` comes the start of the text or any character that is no tag character and none of `#`, `/`, `\` and `&`: a space of any kind, a line break, punctuation, an emoji or a status mark (ADR 0031). So "C#", "F#m", "page.html#anchor" and "example.org/#top" are no tags, nor is the Markdown escape "\#" or an HTML character reference such as "&#x27;".
- A combining mark is no letter, so a "ü" written as "u" and a combining diaeresis ends a tag; keyboards write "ü" as one character.

Offsets are each core's own, UTF-16 code units in the Dart core and UTF-8 bytes in the Rust core, and run from the `#` to the end of the tag. The tag as searched is the `#` and the tag's characters in lower case, each lowered on its own as the Dart core's `toLowerCase` does it, so "İ" becomes "i" and a closing "Σ" "σ" in both cores. The search lowers its query and the texts it searches the same way in both cores. No other normalisation is done. The fixture `text_tags.json` compares the tags as written, as searched and where they lie, counted in code points.

## Searching for a tag

The search stays as it is: a query is found in a text as typed, ignoring case, in the text of a Note and of its Sections, in Transcripts, Photo Texts, Tally entries and the Lines of a Checklist or a Shopping List. A tap on a tag opens the search with the tag as written. So "#Garten" finds every Note holding "#garten" in any case, and not one that only says "Garten". It also finds "#Gartenarbeit", which begins with it, and a "#garten" that is no tag, such as in "C#garten"; finding a little more is better for a memory aid than missing a Note, and typing "#gar" finds tags as they are typed.

## Considered Options

- Searching for a tag as a whole tag, so that "#garten" does not find "#Gartenarbeit" (rejected: a query being typed would find nothing until the tag is complete, and a Note filed under a longer tag would be missed).
- Numbers as tags (rejected: "#126" and "Gleis #3" are numbers people write, not labels).
- Only `_` and no `-` in tags (rejected: German compounds and "to-do" are written with `-`).

## Consequences

- Tags need no new field, no new format and no migration, and Sent Items, Bundles and Backups carry them as text.
- Tags show in every text the app draws: Note texts and Section texts, on cards and in the editor's view, on the phone and the desktop.
