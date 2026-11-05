# Formatted text is a subset of Markdown, kept as written

Notes and Sections can hold formatted text: bold, italic, strike-through, two heading levels, bulleted and numbered lists one level in, quotes, links and inline code (#127), tables (#128) and fenced code blocks (#212). Both cores read it the same way into lines and their parts (`markdown.dart`, `markdown.rs`), so the phone and the desktop draw the same, and the editors' helpers live there too.

## Stored as Markdown in the text marker

A formatted text is stored as the text marker of ADR 0018 holds any text: `{"format":"markdown","text":…}`, keys sorted and no spaces, the same bytes from both cores. The text is Markdown source exactly as written and is never rewritten: what a Device reads it as is worked out each time it is read, so the source round-trips byte for byte, and whatever the subset does not know stays as the characters it is. No key is added, so the structure stays version 1 and no `v` is written. A Device of 0.1.x does not know the format: it shows the source read-only and overwrites nothing (ADR 0018).

## Plain until the first format, formatted from then on

A text stays plain until it is first formatted; the editor decides when: a format applied from the format bar, a shortcut typed (`Markdown.shortcutAt`), or a pasted text that uses a format (`Markdown.uses`). It is then formatted for good. An edit keeps the text's format, an edit asking for plain keeps a formatted text formatted, and making a Note a Checklist or a Shopping List and back keeps the format of its text. The one way out is a Code text (#213, ADR 0018): a formatted text may be made code, its source kept byte for byte, and a code text plain or formatted again; a pasted script such as one whose comments start with `# ` reads as using a format, so a script pasted at Capture is often formatted already when it is made code. In the Note editor a pasted text that looks like code does not format the text; the editor offers to keep it as code instead. The plain text is taken as Markdown as it is, not escaped, so a list typed with "- " becomes a list.

## The subset

Every line is a block of its own, and a line break always shows as one, so text typed as plain lines reads the same once formatted. A line is, the first that fits:

- a heading: up to three spaces, `#` or `##`, then a space, a tab or the line's end;
- a quote: up to three spaces and `>`, one space or tab after it belonging to the mark;
- a bulleted item: any spaces and tabs, `-`, `*` or `+`, then a space, a tab or the line's end;
- a numbered item: any spaces and tabs, one to nine digits and `.`, then a space, a tab or the line's end;
- a paragraph line otherwise, its spaces at the start included.

Spaces count one and a tab four; an item indented by two or more is one level in, a deeper one too. A numbered item right after one of its level, with only items further in between, shows the number after that one's, so "1." on every line counts 1, 2, 3; any other shows its own.

Within a line, never across lines: `\` before ASCII punctuation shows that character; a run of backticks up to the next run of as many is code, shown as it is, one space at both ends dropped when there is one at both; `[label](destination)` is a link, its destination without spaces and with any parentheses balanced, and no link inside a link; `*` and `_` are emphasis by CommonMark's delimiter rules, one for italic and two for bold, with ASCII punctuation as punctuation and the line's start and end as space; a run of exactly two `~` is strike-through. A `[` right after a `!` is no link, so an image shows as written.

Shown as written: `###` and deeper headings, indented code blocks, images, raw HTML, character references, `<…>` autolinks (the web address inside one is a link, as below), reference links, link titles, Setext headings, a rule written as `---`, lists numbered with `)`, and a line that only CommonMark's continuation would put into the item or paragraph above. Items deeper than one level show one level in. The fixture `markdown.json` compares the lines and parts both cores read, offsets counted in code points.

## Tables

A table is GFM's: a header row, the delimiter row right under it, and every line under those that is a row too, each row a line of its own and every one of those lines a block of the table (`MarkdownTables`, `markdown::table`). A line is a row when it would be a paragraph line, is indented by at most three, is not blank, and is no comment (`<!--`), no code block's fence (below) and no rule (three `-`, `*` or `_` and spaces); so a heading, a quote, an item or a blank line ends a table, as GFM has it, and so does a fence or a rule. A row needs no pipe, so a line typed right under a table is a row of it. The delimiter row holds a `|`, its cells are `-` with a `:` at either end telling the alignment (`:--` left, `:-:` center, `--:` right), and it has as many cells as the header row, or neither line is a table's.

A row is split into cells at each `|` that does not follow a `\`, a `|` at its start or end opening or closing it; code does not keep a pipe in. A cell is its content without the spaces and tabs around it, read as one line of the formats above with the cell's edges as the line's; `\|` shows `|`, in code too. A row with fewer cells than the header row gets empty ones; cells beyond the header row's show nowhere but stay in the text.

### The first column as headings

GFM has no way to say that a table's first column holds headings. The line `<!-- first column headings -->` right before the header row says it: an HTML comment, which a Markdown viewer such as GitHub's shows nowhere, and which reads as what it says on a Device of 0.1.x that shows the source. It belongs to the table, so it shows as nothing in the formatted view and the visible text; alone it is a line as written.

### Edits

The table editor reads a cell as written, `\|` as `|`, and writes it with each `|` as `\|` and each line break as a space, so a cell stays one cell of one line whatever is typed into it. An edit writes the whole table anew, every other character of the text kept: each row between pipes, `| a | b |`, the delimiter row `| --- | :-: |`, a cell's content as it was written, the comment line first when the first column holds headings, and the line break that followed the header row. Every line begins with a `|`, so no cell can make a line a heading, a quote or an item. The rows and columns are in the order shown, the header row first: a row put in at the top becomes the header row, deleting the header row makes the next row the header row, and deleting the last row or column deletes the table with its line break. A column's alignment is set to none, left, center or right (`MarkdownTables.setAlign`, `markdown::table::set_align`); setting the one it has changes nothing. A new table has two empty columns, a header row and one more row, and stands on lines of its own with a blank line between it and the lines around it, so it neither joins a table above it nor takes the line under it as a row.

The editors on both apps edit a table as a grid in place of its lines (#257), through these edits and nothing else, so the Markdown stays the source and every change is written back as the table's lines. Column widths are the view's only: Markdown has no widths, so a width dragged on the desktop lasts while the editor shows the table and is never written into the text.

### Reading

A table's rows read as their cells that show something, a space between them; its delimiter row and comment line go with their line break. Search, previews and alarms see that. A Box (ADR 0035, which replaced the status marks of ADR 0031) counts in a table's cell as anywhere else in the text. A line format leaves a table's lines as they are. `Markdown.escape` writes every `|` as `\|`, so an escaped text has no table.

## Code blocks

A fenced code block is CommonMark's, read before tables and the lines above (`MarkdownCodeBlock`, `markdown::CodeBlock`). A line of up to three spaces and a run of at least three backticks or three tildes opens it; after a run of backticks no backtick may follow on the line, or it is no fence. What follows the run, without the spaces and tabs around it, is the info string, kept as written; its first word is the language, kept for a highlighter later and shown nowhere. The block runs to the first line that closes it: up to three spaces, a run of the same character at least as long as the opening one, and only spaces and tabs after it. So a block opened with four backticks is not closed by three, which are a line of it, and a fence of the other character is a line of it too. Without such a line it runs to the end of the text, a line break at the end making an empty last line of it.

Every line of a block, its fences included, is a block of kind `code` holding the code block, so a line is still a block and the line counts stay as they are. A line between the fences is one part shown as written, its spaces at the start included, none of it read as a format, a table, a tag or a Box; a fence is no part and shows nothing. The formatted view shows the lines in the fixed-width font on the background of code, wrapped, the fences left out; the editors show the fences dimmed as marks and the lines in the fixed-width font. A compact card shows a block's lines one by one, without the fences.

Formats, line formats, a link and indenting leave a block's lines as they are; a new table at a cursor in a block goes after it. `Markdown.codeBlock` (`markdown::code_block`), the format bar's "Code block", puts the lines a selection touches into a block, a table or a block it touches taken whole, with fences of backticks longer than any run of them inside and at least three, on lines of their own with the text's first kind of line break; at an empty line the block is empty with the cursor in it. When every line it touches is a line of one block, it takes that block's fences off with their line breaks, and a block without lines goes whole. A closing fence typed is a shortcut that makes a plain text formatted; an opening one is not, as the block it opens runs to the end of the text until it is closed.

## Visible text, tags and Boxes

The visible text is the text without its marks, its line breaks as they are (`Markdown.plain`, `TextFormats.visible`); a code block's fences go with a line break, as a table's delimiter row does, and its lines are as written. Search looks in it for a formatted text, so "bold" finds "**bold**" and "**" finds no marks; a plain text is searched as written. Alarms and other previews the cores make show it too.

Tags (ADR 0032) and Boxes (ADR 0035, which replaced the status marks of ADR 0031) are found in the source as before, so "- ☐ milk" counts. A GFM task list item, `- [ ] milk` or `1. [x] bread`, is the one thing a formatted text is rewritten for (#263, amended): it is taken in as an item with a box, ☐ or ✅, when the text is saved and as it is typed or pasted in the editors, and written back as a task list item when the text is handed to another app (ADR 0035, `Markdown.taskBoxes` and `Markdown.taskLists`); its `[ ]` is no link and no format either way. In a formatted text, a `#word` inside code, inside a code block, its info string included, or inside a link's destination is no tag (`Markdown.tags`), and a glyph there gets no id when the text is saved (`Markdown.literalRanges`, ADR 0035). In any text, plain or formatted, a `#word` or a glyph inside a bare web address (below) is neither (`Markdown.plainTags`, `TextBoxes.convert`).

## Links

A link's destination is written by whoever wrote the text, who may be another person in a Shared Area or behind a Sent Item. The apps open a link only when `Markdown.mayOpen` says so: `http://`, `https://` and `mailto:`, in any case of ASCII letters; anything else, a file, a script or another app's address, stays text.

A bare web address is a link too (#222, amended): in every text a Note shows, plain or formatted, its text, its Sections and Amends, on cards and in the read views, it is drawn as a link and opens through `Markdown.mayOpen` like one. Nothing is stored or rewritten for it; it is how the text is drawn. It starts with `http://` or `https://`, in any case of ASCII letters, not right after an ASCII letter or digit, and an ASCII letter or digit or a character beyond ASCII follows; `www.` alone, `mailto:` and other schemes stay text. It runs up to a space of any kind (Unicode's White_Space), a `<`, a `>` or the end of the text, then, as GitHub's autolinks end, loses its last character while that is one of `.,:;!?`, or a `)` with more `)` than `(` in it. In a formatted text it is found only in what a part shows as written (`Markdown.autolinks`): none in code, a code block, a Code text (#213) or a link, its label included, and an escape or a mark ends it, so `**https://x.org**` links `https://x.org`. A plain text's are `Markdown.plainAutolinks`. Both cores find the same characters; `markdown.json` holds them for every text, formatted and as a plain text, with the plain text's tags.

## Transcripts and Photo Texts

Transcripts and Photo Texts stay plain (ADR 0018). Replacing a formatted Note's text with one, or appending one, writes it escaped (`Markdown.escape`, `Notes.takeOver`), so it reads as itself: a `\` before a line's mark, every `*`, `` ` `` and `[`, a `~` next to another, a `_` not between two characters that are neither space nor punctuation, and a `\` before ASCII punctuation. A tag keeps its `_`.

## HTML

A Note handed to another app (#129) carries its Markdown as text and, beside it, HTML for apps that show formatted text (`Markdown.html`, the same bytes from both cores). It is drawn from the parts above and nothing else: a row of paragraph lines is one `<p>` and a row of quote lines one `<blockquote>`, with `<br>` between the lines; a line of only spaces and tabs ends either; headings are `<h1>` and `<h2>`; a row of items is one `<ul>` or `<ol>` (`start` when it does not begin at 1) with those one level in as a list inside the item above; a table is a `<table>` with its header row in `<thead>`, its heading column as `<th>` and its alignment as `align`; a code block is a `<pre><code>`, `class="language-…"` its language when it has one, each of its lines followed by a line break; bold, italic, strike-through and code are `<strong>`, `<em>`, `<del>` and `<code>`. Every character of the text is escaped, so nothing a person wrote becomes markup, and a link is an `<a>` only when `Markdown.mayOpen` allows it, else its label alone. A Box is its glyph there; the Markdown beside it writes an item's Box as a task list item (ADR 0035, #263). A plain text or a Transcript beside a formatted one goes into the HTML escaped (`Markdown.escape`), so it reads as itself.

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
- A code block closed by any fence of its character, or by three backticks whatever opened it (rejected: CommonMark's rule lets a block hold a shorter fence, as a block about Markdown needs, and other viewers read it so).
- Taking the opening fence's indentation off the block's lines, as CommonMark does (rejected: the lines show as written, the same as the editor shows them).
- A code block as one block of many lines (rejected: every line a block keeps the line counts, the cursor's line and the edits as they are).
- Highlighting by the language now (rejected for now: the language is kept, so a highlighter can come later).

## Consequences

- A Device of 0.1.x shows a formatted text as its source, read-only; a Conflict over it is resolved on a newer Device (ADR 0018).
- A plain text holding "2*3*4" or "- " lines changes how it reads when it is first formatted; nothing of it is lost.
- A text holding a line of three backticks or tildes changes how it reads at this version: what follows reads as code up to a closing fence, or to the end of the text; nothing of it is lost. Indented code blocks show as written. A table edited on a Device of this version is written in its one form, so its spacing changes with the first edit; nothing in its cells changes.
- A line typed right under a table without a blank line becomes a row of it, as in GFM.
