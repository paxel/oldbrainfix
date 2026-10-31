# Imports make dated Notes in a new Area

People come from other note apps with their notes in a folder of Markdown or text files (#150, grill Q435–Q437). An Import makes Notes of them: one Note per file, each dated by its file, all in a new Area named after the source, none of them Unsorted or Nudged. The apps pick the folder with the system's picker and read its files; both cores do the rest the same way (`imports.dart`, `imports.rs`), so the same files make the same entries, byte for byte (the fixture `import`).

## Two steps

An Import takes a list of files, each a name, a modification time (none when the app cannot tell) and its bytes (`Imports.folder`, `Imports::folder`). It reads each file into a Note to make: a text, its format, a date and the file's name. A second step writes such Notes into a new Area and says what it made (`Imports.notes`, `Imports::notes`): the new Area's id, none when nothing was made, the count of Notes made and the count of files skipped, as no text or as too large. The import from Google Keep (#151) reads its export into the same Notes, with what Keep has beyond a text, and writes them through the same second step.

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

## The AI port

An Import has no AI port tool (ADR 0026): it is a person's own act with the system's file picker on files the app reads, not something to set up, and the Notes it makes are read and changed through the tools Notes already have.

## Considered Options

- Reading bytes that are no UTF-8 with replacement characters (rejected: the letters outside ASCII of an older Windows file would be lost, and the two cores' decoders need not replace a broken sequence with the same number of characters) or skipping the file (rejected: its text would not arrive at all).
- Keeping the text without its file's name (rejected: a note whose title lived only in its file name would lose it).
- Dating the entries now and only `captured` by the file (rejected: a Note's entries would say it was written after it was captured, which Capture never writes).
- Filing into an Area of the same name when there is one (rejected: the imported Notes could no longer be told from the user's own, nor be deleted with their Area).
- Leaving the Notes Unsorted to be sorted (rejected: every file of a folder would be Nudged at once; the decision was a new Area, not Nudged).
- Converting what the subset does not know, such as task lists into Boxes or front matter into a Capture Context (rejected for now: the Markdown stays as written, and nothing of it is lost).

## Consequences

- Importing the same folder twice makes two Areas with the same Notes; the second one's name has " (2)" after it.
- A file's name stands over its text as the first line when the text does not begin with it.
- The `import` fixture holds a folder of every kind of file above, the Area beside one of the same name, a folder of nothing usable and a second Import; both cores write its log byte for byte.
