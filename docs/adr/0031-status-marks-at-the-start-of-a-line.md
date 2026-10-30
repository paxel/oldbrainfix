# Status marks are characters of the text, found at the start of a line

Superseded by ADR 0035: the marks never shipped and are replaced by Boxes, which carry an id in the text and are ticked beside it.

For status marks in documentation, such as "v2 supported at customer" with a tick in front (#125), a ☐, ✅ or ❌ at the start of a line of a Note's or a Section's text can be tapped or clicked to change it: Check, Fail and Open, the current state left out. Both cores find and change the marks the same way (`text_marks.dart`, `text_marks.rs`).

## The marks are the text

A mark is the character itself, kept in the text: ☐ (U+2610) open, ✅ (U+2705) checked, ❌ (U+274C) failed. Nothing is stored beside the text, so a mark is signed, synced, judged by the roles of a Shared Area (ADR 0013), won by latest wins and made a Conflict with the rest of the text (ADR 0011), and a text with marks reads the same on a Device that does not know them. Changing a mark is an edit of the text like any other: in a Shared Area only the Note's author and the Managers change it, and a Member's edit of another person's Note is ignored on every Device, as any of their edits of it is. Other boxes and ticks (☑, ☒, ✔, ✓, ❎) are no marks; "✓ " stays the mark that "Back to a plain Note" writes for a checked Line (ADR 0030).

## Where a mark counts

A mark counts only as the first character of a line, so one in the middle of a line stays plain text. Before it may come:

- any spaces and tabs, so an indented line counts; no other kind of space, such as a no-break space or a byte order mark, so both cores agree on every text;
- after those, one list item's bullet, `-`, `*` or `+`, then at least one space or tab, then any more of them, so "- ☐ x" counts in plain text and in Markdown (#127). A bullet without a space after it, a second bullet, a numbered item ("1. ☐") and a quote ("> ☐") do not; #127 may widen this for formatted text.

In a formatted text, a mark at the start of a table's cell counts too, one per cell, the same variation selector and joiner rules applying within the cell (#128, ADR 0033).

Lines break at `\n`, `\r\n` and a lone `\r`, as the Lines of a Checklist are split, and are counted from 0. One variation selector right after the mark, U+FE0E or U+FE0F, belongs to it, as some keyboards add one; a zero-width joiner after the mark, or after its selector, makes it part of another emoji, which is no mark. Any other character may follow, a space or not.

## Changing and inserting

Changing a mark replaces the mark, its variation selector included, with the bare character of the new state and leaves every other character of the text as it was. Nothing changes when the line has no mark or already shows that state. The editor's row, and the format bar of formatted text later, puts "☐ " at the start of the cursor's line, after its spaces, tabs and bullet, where it then counts; on a line that has a mark already it does nothing. A cursor between the `\r` and the `\n` of a line break is on the line before.

Offsets are each core's own: UTF-16 code units in the Dart core, UTF-8 bytes in the Rust core. The fixture `text_marks.json` compares the marks as written, the texts after every change and insert, and the line of every offset counted in code points.

## Consequences

- Marks need no new field, no new format and no migration, and Sent Items, Bundles and Backups carry them as text.
- A mark is changed through the text, so two people changing marks on different lines of the same Note apart make a Conflict of the text, as any two edits of it do.
- Section texts have no edit in this version, so their marks show but cannot be changed until Sections can be edited.
