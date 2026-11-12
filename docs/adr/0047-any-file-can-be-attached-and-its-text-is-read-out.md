# Any file can be attached to a Note, and its text is read out where its format allows

A second brain takes in what life brings: a letter as a PDF, a lease as a Word document, a table of costs, a `.zip` (#267, Q616, Q624, Q625, Q645). Any file can be attached to any Note, as photos and recordings are, and a merged Note's files travel with it into its Section. There is no Note kind for documents: a letter and what the user wrote about it stay one Note.

## The file and its field

The bytes are a blob kept by their SHA-256 in a `files` directory beside the database (`blobs.rs`, `blobs.dart`), as they are: unlike a photo's, nothing is stripped. One field per file on the Note, `file:<hash>`, holds what the attaching Device wrote down as JSON, keys sorted: `{"computer":true,"name":…,"pages":n,"size":n,"type":…}`, `computer` only when true and `pages` only when they were counted. It carries no `v`: an absent one is version 1 (ADR 0034). Files attached on two Devices at once are both kept, as photos are. Null means the file was removed while its text stays (ADR 0020).

`file` is a blob kind beside `photo` and `audio` (`BLOB_KINDS`, `Sync.blobKinds`), so the bytes travel as photos do: over the local network, the shared folder and bundles, in Backups, in area offers, and they are let go when the log no longer refers to them (ADR 0020) or when a person is blocked (ADR 0021). Every Device keeps every file. A file larger than 256 MiB, the most a bundle, a Backup and now the local network carry of one (ADR 0003), is refused when it is attached, and nothing is written. A type not named at attaching, or `application/octet-stream`, is the one the name's ending says.

## Read as data

Another person's Device may have written the field, so both cores read it as data only (`Attachment.read`, `Attachment::read`), and the same rules clean what a Device writes (`cleanName`, `cleanType`; `clean_name`, `clean_type`):

- the name is the part after its last `/` or `\`, without control characters and the marks that turn the direction of text (U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069, which could make `exe.pdf` of `fdp.exe`), its blanks at the ends trimmed, at most 255 characters; `.` and `..` are no name;
- the media type is lower-cased without its parameters when it has the shape `type/subtype` of letters, digits and `!#$&^_.+-`, at most 127 characters, and `application/octet-stream` otherwise;
- the size and the pages count only as whole numbers from 0 to 2^53, what a double holds exactly, so both cores read the same; `computer` only as JSON `true`;
- a value that is no JSON object gives a file without name, type or size, whose bytes still sync.

The field is an edit of the Note like any other: in a Shared Area a Member's file on another person's Note is ignored on every Device (ADR 0013).

## The Attachment Text

The text read out of a file is its Attachment Text, `fileText:<hash>`, kept next to the file as a Photo Text is: absent while not read, empty when nothing was found, null when removed while the file stays (ADR 0025). Only the machine writes it, and it stays plain. Search finds it, and the file's name, as it finds Photo Texts. Both cores tidy it alike (`FileTexts.tidy`, `file_text::tidy`): line breaks as `\n`, no control characters but tabs and line breaks, no blanks at the ends of lines, at most one blank line in a row, cut after 1,000,000 characters.

What a file is follows from the ending of its name, in any case, else from its media type (`FileTexts.kindOf`, `file_text::kind_of`):

- `.docx`: the paragraphs of `word/document.xml`, each on a line, with its tabs and line breaks; deleted text and field codes are left out, headers and footers too.
- `.xlsx`: each sheet under its name in the workbook's order, each row with a value on a line, its cells' values between tabs: shared strings by their index without their phonetic guides, inline strings, truth values as TRUE and FALSE, anything else as written.
- `.odt`: the paragraphs and headings of `content.xml`, with their spaces, tabs and line breaks; tracked deletions are left out.
- `.ods`: each sheet under its name, each row with a value on a line, its cells' texts between tabs; a cell or row repeated is written out at most 100 times, and empty ones at a row's end or rows without a value not at all.
- `.txt`, `.text`, `.md`, `.markdown`, `.csv`, `.tsv`: read as an Import reads a text file (ADR 0036): UTF-8 without its byte order mark, else Windows-1252; a file holding a zero has no text.
- `.pdf`: below.
- Anything else, a `.zip` or a `.dwg`: kept without text, opened with the system's app.

The office files are zips; their parts are read with the same small XML reader in both cores, which resolves references, takes CDATA as text and stops at the first tag that does not end, a part up to 32 MiB and 256 sheets. A file that does not read as its kind says, a damaged zip or a part that is no UTF-8, gives an empty text, so it does not wait to be read for ever. The fixture `file_texts.json` holds every case.

## PDF: one reader, in Rust

A PDF is read by `oldbrainfix-pdf` (`desktop/crates/oldbrainfix-pdf`), pure Rust on `flate2` and `crc32fast`, which the desktop core re-exports as `oldbrainfix_core::pdf`. It finds the objects by scanning the file rather than by its cross-reference table, so an updated or damaged file still reads, steps over stream data while it scans, and reads object streams, the FlateDecode (with PNG predictors), ASCIIHexDecode and ASCII85Decode filters and the page tree with inherited resources. Each page's content is run for its text: through the fonts' ToUnicode maps, else for a simple font its WinAnsi, MacRoman or Standard encoding with the glyph names of its differences; a line break where the text moves down more than half its size, a space where it moves on further than 0.15 of its size; forms drawn by the page run too. An encrypted PDF gives no text; its pages are still counted. A crafted file must not exhaust the memory: a stream is decoded to at most 64 MiB through at most eight filters, a page's content and the forms it draws to 64 MiB each, the object streams to 64 MiB together, and arrays and dictionaries keep at most 2^20 items between two operators or in one object, 64 levels deep; a picture larger than 64 MiB of pixels is no scan.

A page with fewer than four characters other than blanks whose largest picture has at least 200 by 200 pixels is a scan: its picture is handed on for the OCR, a JPEG as it lies in the PDF, gray, colour or black-and-white pixels as a PNG. The app reads it with RustO as it reads a photo (ADR 0007) and joins the page texts, a blank line between them (`FileTexts.joinPages`, `file_text::join_pages`). Scans in JPEG 2000, JBIG2 or CCITT fax are not read; such a page gives no text.

The Dart core has no PDF reader of its own: `FileTexts.read` gives `PdfToRead`, and the phone is to read a PDF through the same crate, built into its native library beside RustO (`android/ocr`, the iPhone's bridge), once the app does; until then a phone attaches every PDF with its pages not counted, so a computer reads it. A second reader written in Dart would disagree with this one on the many PDFs no fixture holds, and the same file would give different texts on the phone and on the computer.

## Who reads the text (Q645)

The Device that attaches a file decides who reads it and writes it down: `computer` is true for a PDF of more than 10 pages, or whose pages were not counted, and for any other file with text larger than 10 MiB (`FileTexts.computerReads`, `file_text::computer_reads`). The app counts a PDF's pages and hands them in (`NewFile.pages`; on the desktop `pdf::page_count`). A phone reads only the others (`unreadAttachments(computer: false)`); a computer reads every file waiting (`unread_attachments(true)`), and its text syncs back like any other. Until then the phone shows that the text follows from the computer. Written down, the decision stays the same on every phone, whatever limit a later version sets; the phone's speed was never measured, so the limit is set once it is.

## Removing, roles, Packs and the AI port

- The Attachment Text can be removed and the file stays (ADR 0025); a file with a text that is not empty can be removed and its text stays (ADR 0020). A file without a text cannot be removed this way, as a photo cannot: the Note would lose all of it.
- A Member's machine adds Attachment Texts to any Note of a Shared Area; a null under `fileText:` is judged like any other edit, so only the Note's owner or a Manager removes one (ADR 0025).
- A Note made from a file alone shows the file as its card (`Note.aloneFile`, `Note::alone_file`): its only file, while its text is blank and it has no photo, recording or other file, removed ones included.
- A Pack lists each file and its Attachment Text as items of their own (`n<i>.d<j>`, `n<i>.dt<j>`), the texts ticked at the start and the files not, as for photos (ADR 0043). The payload's `attachments` holds `{"bytes","hash","name","pages","text","type"}`; the receiver writes the field with its own rule's `computer`. Taking Notes back from a Backup reads their files as it reads photos (#285).
- A Sent Item carries no file, as it carries no photo or recording (ADR 0009).
- The AI port's `read_note` lists each file with its hash, name, media type, size, pages, Attachment Text (cut after 4000 characters, its whole length beside it) and what became of the text; `read_attachment_text` reads on from a character, at most 50,000 at once (ADR 0026). No tool hands out a file's bytes, attaches a file or writes its text.

## Considered Options

- A Note kind for documents (rejected by Q625: a letter and what the user wrote about it are one Note).
- Accepting only the formats whose text is read (rejected by Q624: a refused file goes to another tool and is lost there).
- A PDF reader in each core (rejected: see above), or a library of the system (rejected: the phones' and the desktops' systems read PDFs each their own way, and a Linux desktop has none to rely on).
- Rendering scanned pages to pictures (rejected for now: no pure-Rust renderer was at hand, and a scan's page is almost always one picture, which is handed on as it lies).
- Fetching a file's bytes only when it is opened (rejected by Q616: with no server, nothing opens while the computer is off).

## Older versions

A version before this one ignores both fields: its Notes show without the files, it neither asks for their bytes nor passes them on, and a Pack's files stay out when it opens one. It refuses a Backup holding files as damaged, as it refuses any blob of a kind it does not know.

## Consequences

- A file another Device attached arrives with the next sync that brings its bytes; until then the Note lists it without them (`Notes.fileBytes` gives null, `Notes::has_file` false).
- A file that went into a Pack without its bytes but with its text arrives with its text and without its name.
- A large scanned PDF attached on a phone has no text until a computer has read it; with no computer, it has none.
