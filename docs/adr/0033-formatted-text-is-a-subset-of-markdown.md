# Formatted text is a subset of Markdown, kept as written

Notes and Sections can hold formatted text: bold, italic, strike-through, two heading levels, bulleted and numbered lists one level in, quotes, links and inline code (#127), and tables (#128). Both cores read it the same way into lines and their parts (`markdown.dart`, `markdown.rs`), so the phone and the desktop draw the same, and the editors' helpers live there too.

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

Shown as written: `###` and deeper headings, code blocks, images, raw HTML, character references, autolinks, reference links, link titles, Setext headings, a rule written as `---`, lists numbered with `)`, and a line that only CommonMark's continuation would put into the item or paragraph above. Items deeper than one level show one level in. The fixture `markdown.json` compares the lines and parts both cores read, offsets counted in code points.

## Tables

A table is GFM's: a header row, the delimiter row right under it, and every line under those that is a row too, each row a line of its own and every one of those lines a block of the table (`MarkdownTables`, `markdown::table`). A line is a row when it would be a paragraph line, is indented by at most three, is not blank, and is no comment (`<!--`), no fence (three backticks or tildes) and no rule (three `-`, `*` or `_` and spaces); so a heading, a quote, an item or a blank line ends a table, as GFM has it, and so does a fence or a rule. A row needs no pipe, so a line typed right under a table is a row of it. The delimiter row holds a `|`, its cells are `-` with a `:` at either end telling the alignment (`:--` left, `:-:` center, `--:` right), and it has as many cells as the header row, or neither line is a table's.

A row is split into cells at each `|` that does not follow a `\`, a `|` at its start or end opening or closing it; code does not keep a pipe in. A cell is its content without the spaces and tabs around it, read as one line of the formats above with the cell's edges as the line's; `\|` shows `|`, in code too. A row with fewer cells than the header row gets empty ones; cells beyond the header row's show nowhere but stay in the text.

### The first column as headings

GFM has no way to say that a table's first column holds headings. The line `<!-- first column headings -->` right before the header row says it: an HTML comment, which a Markdown viewer such as GitHub's shows nowhere, and which reads as what it says on a Device of 0.1.x that shows the source. It belongs to the table, so it shows as nothing in the formatted view and the visible text; alone it is a line as written.

### Edits

The table editor reads a cell as written, `\|` as `|`, and writes it with each `|` as `\|` and each line break as a space, so a cell stays one cell of one line whatever is typed into it. An edit writes the whole table anew, every other character of the text kept: each row between pipes, `| a | b |`, the delimiter row `| --- | :-: |`, a cell's content as it was written, the comment line first when the first column holds headings, and the line break that followed the header row. Every line begins with a `|`, so no cell can make a line a heading, a quote or an item. The rows and columns are in the order shown, the header row first: a row put in at the top becomes the header row, deleting the header row makes the next row the header row, and deleting the last row or column deletes the table with its line break. A new table has two empty columns, a header row and one more row, and stands on lines of its own with a blank line between it and the lines around it, so it neither joins a table above it nor takes the line under it as a row.

### Reading

A table's rows read as their cells that show something, a space between them; its delimiter row and comment line go with their line break. Search, previews and alarms see that. A Box (ADR 0035, which replaced the status marks of ADR 0031) counts in a table's cell as anywhere else in the text. A line format leaves a table's lines as they are. `Markdown.escape` writes every `|` as `\|`, so an escaped text has no table.

## Visible text, tags and Boxes

The visible text is the text without its marks, its line breaks as they are (`Markdown.plain`, `TextFormats.visible`). Search looks in it for a formatted text, so "bold" finds "**bold**" and "**" finds no marks; a plain text is searched as written. Alarms and other previews the cores make show it too.

Tags (ADR 0032) and Boxes (ADR 0035, which replaced the status marks of ADR 0031) are found in the source as before, so "- ☐ milk" counts. In a formatted text, a `#word` inside code or inside a link's destination is no tag (`Markdown.tags`).

## Links

A link's destination is written by whoever wrote the text, who may be another person in a Shared Area or behind a Sent Item. The apps open a link only when `Markdown.mayOpen` says so: `http://`, `https://` and `mailto:`, in any case of ASCII letters; anything else, a file, a script or another app's address, stays text.

## Transcripts and Photo Texts

Transcripts and Photo Texts stay plain (ADR 0018). Replacing a formatted Note's text with one, or appending one, writes it escaped (`Markdown.escape`, `Notes.takeOver`), so it reads as itself: a `\` before a line's mark, every `*`, `` ` `` and `[`, a `~` next to another, a `_` not between two characters that are neither space nor punctuation, and a `\` before ASCII punctuation. A tag keeps its `_`.

## HTML

A Note handed to another app (#129) carries its Markdown as text and, beside it, HTML for apps that show formatted text (`Markdown.html`, the same bytes from both cores). It is drawn from the parts above and nothing else: a row of paragraph lines is one `<p>` and a row of quote lines one `<blockquote>`, with `<br>` between the lines; a line of only spaces and tabs ends either; headings are `<h1>` and `<h2>`; a row of items is one `<ul>` or `<ol>` (`start` when it does not begin at 1) with those one level in as a list inside the item above; a table is a `<table>` with its header row in `<thead>`, its heading column as `<th>` and its alignment as `align`; bold, italic, strike-through and code are `<strong>`, `<em>`, `<del>` and `<code>`. Every character of the text is escaped, so nothing a person wrote becomes markup, and a link is an `<a>` only when `Markdown.mayOpen` allows it, else its label alone. A plain text or a Transcript beside a formatted one goes into the HTML escaped (`Markdown.escape`), so it reads as itself.

## Considered Options

- A Markdown library in each core (rejected: two libraries never read the same text into the same parts, and the cards of both apps must agree).
- CommonMark's soft line breaks, a single line break shown as a space (rejected: a text typed line by line would run together the moment it is first formatted).
- A rich text model, such as spans with offsets in JSON (rejected: a Device of 0.1.x shows the source, and Markdown reads as text where JSON does not).
- Escaping the plain text when it is first formatted (rejected: backslashes would appear in the source, and a list typed with "- " would stay dashes).
- A new structure version `v` (rejected: no key is added, a new format name is enough).
- The first column as headings written as a bold first column (rejected: a user's own bold cells would turn the flag on, and an empty cell cannot be bold), as a count of hyphens in the delimiter row (rejected: invisible, and people type any count), or as an attribute such as `{.headings}` (rejected: GitHub and most viewers show it as text).
- Keeping a table's spacing as written and changing only the cells an edit touches (rejected: putting in or moving a column changes every row anyway, and one written form is the same bytes from both cores).
- Writing a table with its columns padded to one width (rejected: the widths of characters differ in the fonts the apps use, and one long cell would pad every row).
- Dropping the cells beyond the header row's when a table is written anew, as GFM ignores them (rejected: nothing a person wrote is lost by an edit).

## Consequences

- A Device of 0.1.x shows a formatted text as its source, read-only; a Conflict over it is resolved on a newer Device (ADR 0018).
- A plain text holding "2*3*4" or "- " lines changes how it reads when it is first formatted; nothing of it is lost.
- Code blocks show as written. A table edited on a Device of this version is written in its one form, so its spacing changes with the first edit; nothing in its cells changes.
- A line typed right under a table without a blank line becomes a row of it, as in GFM.
