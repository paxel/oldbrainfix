# Formatted text is a subset of Markdown, kept as written

Notes and Sections can hold formatted text: bold, italic, strike-through, two heading levels, bulleted and numbered lists one level in, quotes, links and inline code (#127). Both cores read it the same way into lines and their parts (`markdown.dart`, `markdown.rs`), so the phone and the desktop draw the same, and the editors' helpers live there too.

## Stored as Markdown in the text marker

A formatted text is stored as the text marker of ADR 0018 holds any text: `{"format":"markdown","text":…}`, keys sorted and no spaces, the same bytes from both cores. The text is Markdown source exactly as written and is never rewritten: what a Device reads it as is worked out each time it is read, so the source round-trips byte for byte, and whatever the subset does not know stays as the characters it is. No key is added, so the structure stays version 1 and no `v` is written. A Device of 0.1.x does not know the format: it shows the source read-only and overwrites nothing (ADR 0018).

## Plain until the first format, formatted from then on

A text stays plain until it is first formatted; the editor decides when: a format applied from the format bar, a shortcut typed (`Markdown.shortcutAt`), or a pasted text that uses a format (`Markdown.uses`). It is then formatted for good. An edit keeps the text's format, an edit asking for plain keeps a formatted text formatted, and making a Note a Checklist or a Shopping List and back keeps the format of its text. The plain text is taken as Markdown as it is, not escaped, so a list typed with "- " becomes a list.

## The subset

Every line is a block of its own, and a line break always shows as one, so text typed as plain lines reads the same once formatted. A line is, the first that fits:

- a heading: up to three spaces, `#` or `##`, then a space, a tab or the line's end;
- a quote: up to three spaces and `>`, one space or tab after it belonging to the mark;
- a bulleted item: any spaces and tabs, `-`, `*` or `+`, then a space, a tab or the line's end;
- a numbered item: any spaces and tabs, one to nine digits and `.`, then a space, a tab or the line's end;
- a paragraph line otherwise, its spaces at the start included.

Spaces count one and a tab four; an item indented by two or more is one level in, a deeper one too. A numbered item right after one of its level, with only items further in between, shows the number after that one's, so "1." on every line counts 1, 2, 3; any other shows its own.

Within a line, never across lines: `\` before ASCII punctuation shows that character; a run of backticks up to the next run of as many is code, shown as it is, one space at both ends dropped when there is one at both; `[label](destination)` is a link, its destination without spaces and with any parentheses balanced, and no link inside a link; `*` and `_` are emphasis by CommonMark's delimiter rules, one for italic and two for bold, with ASCII punctuation as punctuation and the line's start and end as space; a run of exactly two `~` is strike-through. A `[` right after a `!` is no link, so an image shows as written.

Shown as written: `###` and deeper headings, code blocks, tables (#128), images, raw HTML, character references, autolinks, reference links, link titles, Setext headings, a rule written as `---`, lists numbered with `)`, and a line that only CommonMark's continuation would put into the item or paragraph above. Items deeper than one level show one level in. The fixture `markdown.json` compares the lines and parts both cores read, offsets counted in code points.

## Visible text, tags and status marks

The visible text is the text without its marks, its line breaks as they are (`Markdown.plain`, `TextFormats.visible`). Search looks in it for a formatted text, so "bold" finds "**bold**" and "**" finds no marks; a plain text is searched as written. Alarms and other previews the cores make show it too.

Tags (ADR 0032) and status marks (ADR 0031) are found in the source as before, so "- ☐ milk" counts. In a formatted text, a `#word` inside code or inside a link's destination is no tag (`Markdown.tags`).

## Links

A link's destination is written by whoever wrote the text, who may be another person in a Shared Area or behind a Sent Item. The apps open a link only when `Markdown.mayOpen` says so: `http://`, `https://` and `mailto:`, in any case of ASCII letters; anything else, a file, a script or another app's address, stays text.

## Transcripts and Photo Texts

Transcripts and Photo Texts stay plain (ADR 0018). Replacing a formatted Note's text with one, or appending one, writes it escaped (`Markdown.escape`, `Notes.takeOver`), so it reads as itself: a `\` before a line's mark, every `*`, `` ` `` and `[`, a `~` next to another, a `_` not between two characters that are neither space nor punctuation, and a `\` before ASCII punctuation. A tag keeps its `_`.

## Considered Options

- A Markdown library in each core (rejected: two libraries never read the same text into the same parts, and the cards of both apps must agree).
- CommonMark's soft line breaks, a single line break shown as a space (rejected: a text typed line by line would run together the moment it is first formatted).
- A rich text model, such as spans with offsets in JSON (rejected: a Device of 0.1.x shows the source, and Markdown reads as text where JSON does not).
- Escaping the plain text when it is first formatted (rejected: backslashes would appear in the source, and a list typed with "- " would stay dashes).
- A new structure version `v` (rejected: no key is added, a new format name is enough).

## Consequences

- A Device of 0.1.x shows a formatted text as its source, read-only; a Conflict over it is resolved on a newer Device (ADR 0018).
- A plain text holding "2*3*4" or "- " lines changes how it reads when it is first formatted; nothing of it is lost.
- Code blocks and tables are left for #128; until then they show as written.
