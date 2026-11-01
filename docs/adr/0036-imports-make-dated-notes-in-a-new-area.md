# Imports make dated Notes in a new Area

People come from other note apps with their notes in a folder of Markdown or text files (#150, grill Q435–Q437). An Import makes Notes of them: one Note per file, each dated by its file, all in a new Area named after the source, none of them Unsorted or Nudged. The apps pick the folder with the system's picker and read its files; both cores do the rest the same way (`imports.dart`, `imports.rs`), so the same files make the same entries, byte for byte (the fixture `import`).

## Two steps

An Import takes a list of files, each a name, a modification time (none when the app cannot tell) and its bytes (`Imports.folder`, `Imports::folder`). It reads each file into a Note to make: a text, its format, a date and the file's name. A second step writes such Notes into a new Area and says what it made (`Imports.notes`, `Imports::notes`): the new Area's id, none when nothing was made, the count of Notes made and the count of files skipped, as no text or as too large. The import from Google Keep (#151) reads its export into the same Notes, with what Keep has beyond a text (Lines and photos), and writes them through the same second step, which then also counts the Checklists and photos made and what was skipped for reasons of Keep's own (see Google Keep below).

## Files

A file is taken by its extension, in any case of ASCII letters: `.md` and `.markdown` make a formatted Note (ADR 0033), `.txt` a plain one. Any other file is skipped, and so is a hidden one, whose name starts with a `.` (such as the `._` files macOS leaves beside others). Only the part of a name after its last `/` or `\` counts, so an app may hand over files of subfolders by their paths.

The bytes are UTF-8, a byte order mark before them dropped. Bytes that are no UTF-8 are read as Windows-1252, the encoding older text files on Windows are in, which reads every byte, so a letter such as "ü" stays a letter instead of becoming a replacement character; the five bytes Windows-1252 leaves open read as Latin-1 does. A file holding a zero byte is no text file (one in UTF-16 holds them) and is skipped. Line breaks `\r\n` and `\r` become `\n`.

The text is kept as written: Markdown the subset does not know, such as a task list `- [ ]`, front matter between `---` lines, a code block or an image, shows as the characters it is (ADR 0033), and a ☐, ✅ or ❌ becomes a Box as at Capture (ADR 0035). Blank lines at its start and blanks at its end go.

## The title

A Note's first line with words is its title. Many apps keep a note's title only in its file name, so the file's name without its extension becomes the Note's first line, unless the text's first line with words, as it reads without its marks, says it already, in any case: "Dentist.txt" holding "Tuesday 9:00" becomes "Dentist" over "Tuesday 9:00", while "Shopping.md" beginning with "# Shopping" stays as it is. In a formatted Note the name is a heading, escaped (`Markdown.escape`) so that it reads as itself. An empty file thus becomes a Note of its name alone; a file with neither words nor a name is skipped.

## Dates

A Note is captured when its file was last changed, and every entry the Import writes for it is dated so, as Capture dates its entries by the moment it captures; anything done to the Note afterwards is dated later and wins (latest wins, ADR 0002). A file whose time is unknown, at or before 1970-01-01 (as a platform says 0 for one it does not know), or later than now is dated now. Times are kept to the microsecond. The Notes are written oldest first; Notes of the same moment go by their file's name, compared by code points as both cores compare strings, then in the order the app handed them over.

## The Area

The Notes land in a new private Area named as the app says, such as "From Notes" or the folder's name. An Import never mixes its Notes into an Area there already: when an Area has that name, the new one is named with " (2)" after it, or the first of " (3)", " (4)" and so on that is free. The Area's entries are dated now.

The Notes are filed into the Area at once, so none of them is Unsorted and no Nudge comes for them, and no Nudge time is written. A Note of an old date left Unsorted would bring the Nudge at once, for every file of the folder.

## Nothing made

A list without a file to take, an empty folder or one of photos only, makes no Area and writes nothing; the summary says what was skipped.

## Sizes

A file of more than 1 MiB is skipped as too large without being read, and the apps need not read one. A text of more than 256 KiB, as stored with its format, is skipped too: a Backup reads a record of up to 1 MiB, one entry each (ADR 0016, `backup.dart`), the value escaped once more inside it. One Import keeps at most 32 MiB of text, the newest files first, the older ones beyond that skipped as too large.

## Google Keep

People export their notes from Google Keep with Google Takeout (#151, grill Q435–Q437), a zip holding a folder `Takeout/Keep` with one `.json` per note, an `.html` twin of each, `Labels.txt` and the files attached to the notes. The apps hand over the entries of the zip, or the files of the folder once unpacked, each by its path and its bytes (`Imports.keep`, `Imports::keep`); both cores do the rest. The field names follow the export as Keep writes it, checked against the importers of Obsidian and Trilium: `title`, `textContent`, `listContent` (items with `text` and `isChecked`), `labels` (each with a `name`), `attachments` (each with a `filePath`), `isTrashed`, `isArchived`, `createdTimestampUsec` and `userEditedTimestampUsec`.

- **What is read.** A `.json` file is read as a note, any other only when a note names it; the `.html` twins, `Labels.txt` and the rest are not read, and not counted. When any entry lies in a folder named `Keep` (in any case of ASCII letters), only the entries of such a folder count, so a Takeout of several Google products can be handed over whole. Each core says which entries it reads, and how many bytes of them, so the apps need read no more (`Imports.keepReads`, `keep_reads`): a `.json` of up to 1 MiB, a photo of up to 16 MiB. A larger one is handed over as that many bytes and one more, unread, and counted as too large.
- **A note.** A `.json` is a note when its bytes are UTF-8 (a byte order mark before them dropped) holding a JSON object with either time as a whole number; anything else is skipped as no text. A value of the wrong kind counts as absent: a flag is set only by `true`, a text only by a string. A note that holds nothing, neither a text nor a Line nor a photo, is skipped as no text, its labels alone being no text.
- **The trash and the archive.** A note in Keep's trash is skipped and counted as such: it was thrown away, and Keep deletes it after seven days. An archived note is made as any other: Keep's archive only takes a note out of sight, and in Old Brain Fix every imported Note is out of the way already, filed into the new Area and not Nudged. Pinned notes and colours are left out; neither has a meaning here.
- **The text.** The text is plain; the formatting of `textContentHtml` is left out and its words are in `textContent`. The title is the first line, as a file's name is for a folder (The title above), unless the text begins with it. Line breaks `\r\n` and `\r` become `\n`, and a ☐, ✅ or ❌ becomes a Box as at Capture.
- **Labels.** Each label becomes a tag (ADR 0032) on a last line of the text, the tags apart by spaces, in Keep's order (`TextTags.fromLabel`, `text_tags::from_label`): each run of characters that are no letter, digit or `_` becomes one `-`, a `-` at either end goes, and the case stays, so "Home Garden" and "Home & Garden" both become "#Home-Garden". A label of no letter, such as "2024", becomes "#2024", which reads as a number and is no tag, but the search finds it; one of neither letter nor digit, such as an emoji alone, is left out. A tag already in the text, as Keep makes a label of a `#word` typed into a note, and a tag already added are not added again, compared in lower case as the search compares them.
- **Lists.** A note with `listContent` becomes a Checklist (ADR 0030) of its items, an empty list too; the title and the text stay its text. Each item with text is a Line at the top, in Keep's order, its text on one line (Keep's export says nothing of indents); an item checked in Keep is checked here: the Line is written unchecked, then checked by a `checked` entry of its own, as every check is. The kind is written at the note's time and nothing is `spawned`, as for an accepted Sent Item, since the Note is filed into the Area already.
- **Photos.** An attachment whose file ends in `.jpg`, `.jpeg`, `.png`, `.webp` or `.gif` (in any case) is a photo of the Note, kept as a photo captured is: without its metadata, under its hash, each photo once per Note. It is looked up among the entries by the part of its path after the last `/` or `\`; one named `.jpeg` and held as `.jpg`, or the other way round, is found by its other name. One not among the entries is counted as missing; an attachment of another kind, such as a voice recording, as no photo. Neither keeps the note from being made.
- **Dates.** A note is captured when Keep says it was made (`createdTimestampUsec`), else when it was last changed (`userEditedTimestampUsec`), else now; a time at or before 1970 or later than now counts as none. Its entries are dated so, as for a file. Its checks are dated when the note was last changed, since that is the last moment Keep knows of, but never before the note was made.
- **Sizes.** The text and the Lines of a note count together against the 256 KiB a text may have and the 32 MiB an Import keeps; photos count against neither.
- **What it says.** Besides the Area, the Notes, those skipped as no text and as too large, the summary counts the Checklists and the photos made, the notes skipped as in the trash, the files missing and the files that are no photo. Every entry the Import writes is written by this Device, into a new private Area, so nothing in it is trusted from another person; the apps read nothing but the files the person picked.
- **The Agenda.** A Checklist with open Lines shows on today's Agenda (ADR 0030); one imported from Keep starts with "Show on the Agenda" switched off for the person importing, written in the same batch as their own choice (`agenda:<note id>` = `off`, as `Checklists.setShowOnAgenda` writes it), so an Import of many old lists does not fill today's Agenda. The person switches on the ones they want; the switch stays theirs alone, and other people in a Shared Area the Note is filed into later start on, as for any Checklist. A folder makes no Checklists, so nothing of this applies to it.

## The AI port

An Import has no AI port tool (ADR 0026): it is a person's own act with the system's file picker on files the app reads, not something to set up, and the Notes it makes are read and changed through the tools Notes already have.

## Considered Options

- Reading bytes that are no UTF-8 with replacement characters (rejected: the letters outside ASCII of an older Windows file would be lost, and the two cores' decoders need not replace a broken sequence with the same number of characters) or skipping the file (rejected: its text would not arrive at all).
- Keeping the text without its file's name (rejected: a note whose title lived only in its file name would lose it).
- Dating the entries now and only `captured` by the file (rejected: a Note's entries would say it was written after it was captured, which Capture never writes).
- Filing into an Area of the same name when there is one (rejected: the imported Notes could no longer be told from the user's own, nor be deleted with their Area).
- Leaving the Notes Unsorted to be sorted (rejected: every file of a folder would be Nudged at once; the decision was a new Area, not Nudged).
- Converting what the subset does not know, such as task lists into Boxes or front matter into a Capture Context (rejected for now: the Markdown stays as written, and nothing of it is lost).
- Importing the notes in Keep's trash, or leaving the archived ones out (rejected: a note thrown away would come back, and an archived one, kept on purpose, would be lost).
- Keep's labels as Topics or Areas (rejected by the grill: labels are tags, Q435–Q437; a note has many labels and a Note one place).
- Keep's formatted text from `textContentHtml` as a formatted Note (rejected for now: the words are all in `textContent`, and an HTML reader in both cores would have to agree on every tag).
- Keep's voice recordings as recordings of the Note (rejected for now: the ticket asks for photos; they are counted, so the person knows).
- Leaving imported Checklists on the Agenda, as every other Checklist starts (rejected: an Import of dozens of Keep lists with open Lines would put every one of them on today's Agenda at once, much as Nudging every imported Note would).
- Reading every `.json` of a Takeout (rejected: the sidecars of other Google products would be counted as skipped by the hundred).

## Consequences

- Importing the same folder twice makes two Areas with the same Notes; the second one's name has " (2)" after it.
- A file's name stands over its text as the first line when the text does not begin with it.
- The `keep` fixture holds a small Takeout: a note with a title, labels, a photo, one found by its other name, one missing and a voice recording; a list with a checked Line; an archived note; a note in the trash, one that is no JSON, the `.html` twin, `Labels.txt` and a note of another product; both cores write its log and its photos byte for byte.
- The `import` fixture holds a folder of every kind of file above, the Area beside one of the same name, a folder of nothing usable and a second Import; both cores write its log byte for byte.
