# App links open places in the app

A Note can link to another Note, an Area or Topic and the app's own pages, and the help Notes link each Quick start step to its place (#261, Q595–Q599). Both cores read and write these links the same way (`app_link.dart`, `app_link.rs`); the apps open them.

## An ordinary Markdown link with the app's scheme

An app link is a link of a formatted text (ADR 0033) whose destination has the scheme `oldbrainfix://`, in any case of ASCII letters, then one of these, written exactly so:

- `note/<id>` a Note, `area/<id>` an Area or a Topic, by the id it has on every Device;
- `agenda`, `unsorted`, `routines` (All Routines and Reminders), `profile`, `frogs`, `session/new` (New Session);
- `settings`, `settings/<section>` one of the Settings' sections (`look`, `units`, `agenda`, `alarms`, `nudges`, `capture`, `quick-note`, `calendar`, `privacy`, `typing`, `history`, `import`, `features`, `computer`), `settings/backups` and `settings/sync`; since obf-1096 the phone's Imports sit in the Areas list's ⋮, so `settings/import` is no longer offered by Link to… and an old link of it opens the Settings at the top, while the cores still read it;
- `guide` the user guide's start page, `guide/<page>` one of its pages and `guide/<page>#<box>` one of its boxes, the names lower-case letters, digits and `-`;
- `file/<path>` a file of the shared folder by its path inside it.

An id and a file's names are %-encoded but for ASCII letters, digits, `-`, `.`, `_`, `~`, `@` and, in an id, `:`, upper-case hex, so `[Dentist](oldbrainfix://note/note:3f2a)`. No query, no other `#`, no empty or extra part; anything else of the scheme is no app link. Nothing new is stored: the link is part of the text, so it syncs, is shared and goes into Packs as the text does, and the AI port reads it as written. A plain Markdown app shows its label.

## Opening

A link opens without asking. The Note opens in its editor (the phone) or the Note dialog (the desktop), an Area or Topic in the browser, the Agenda as the first page, the pages as they open from their menus, the Settings scrolled to the section where the app has it and at their top where it has not, and a guide page in the browser in the app's language, English where the site has none. A target the reader cannot see says "Not available here": a Note or Area this Device never got, in the trash or merged, a Note of a Shared Area the reader is no member of. A Note in a closed locked Area asks for the PIN first, as opening its Area does; the desktop has no Area Locks. New Session in Simple says it too, as only Advanced makes Sessions (ADR 0041).

## Files: only in the shared folder, after one question

A file link names a path inside the shared folder: on the phone the folder picked for sync (its document tree, walked name by name), on the desktop the synced folder (the path joined and resolved, symbolic links followed, and refused unless it still lies inside). The cores refuse empty names, `.`, `..`, `\`, `:` and control characters, so no link names a place outside the folder or a drive. Before it opens, one question names the file and the app the system opens it with ("Open "Scans/tax.pdf" from the shared folder with Document Viewer?"), "the app the system picks for it" where the system does not tell (a chooser on Android, macOS, Windows, the Flatpak, the iPhone). Nothing is imported, copied or shown in the app. A file the folder does not hold, or no shared folder, says "Not available here". On the iPhone the folder is the one picked in the Files app, kept as a bookmark; the file is found name by name inside it, refused once symbolic links lead out, and handed to the apps that take it through the system's Open In menu, which names no app, so the question says "the app the system picks for it"; a file of iCloud Drive not downloaded yet is not available until it is.

Amended by obf-1151: the Flatpak is no longer built, so it is no longer among the systems that do not tell.

## Picking, not typing

The format bar's "Link to…" opens a picker of the app's pages, the Areas with their Topics and the Notes (by their first line as it reads), with a search over all of them; a Note of a closed locked Area is not offered. The one picked goes on the selection or the word at the cursor, or, without either, in at the cursor with its name as the label (`Markdown.link` with `label`, `markdown::link_labelled`). So nobody types an id. Simple has no format bar (ADR 0041), so no "Link to…" either; links already written open there too.

## HTML and other apps

The HTML handed to another app (#129) writes an app link as its label alone: another app cannot open it, and the system might hand the scheme to this app with nothing to say what to do.

## The help Notes

From 0.5.0 the Quick start and What the icons mean are formatted texts (ADR 0042, amended here): the Quick start's Description first lists, numbered as the Lines are, a link to each step's place (the Agenda, Unsorted, All Routines and Reminders twice, "Here, in this Checklist." for ticking a Line, the Agenda, Backups, Sync), then the desktop's way; What the icons mean ends with a link to the guide's start page. A Checklist's Lines are plain, so the links go into the Description, not into the Lines. Both apps write these texts from the same strings through `Markdown.link`, so the bytes are the same; the fixture `help` holds a link. Help Notes made before stay as they are (ADR 0042).

## Linked from (amended for obf-1035)

A Note shows the Notes that link to it, "Linked from" (#345, Q12, Q21, Q22): the Notes neither in the trash nor merged whose text, or the text of one of their Sections, holds an app link `note/<id>` to it, in a formatted text as it is drawn, a link inside bold, a quote or a table's cell too, the scheme in any case and the id %-encoded or not. A plain or Code text has no links, and a link to an Area, a Topic, a page or a file, or a web link, has none. Each Note is listed once, newest capture first and by id within one capture time, never under itself; a Note merged into another links from there, as its text is a Section of it. Both cores read them alike (`Backlinks`, `backlinks.dart`, `backlinks.rs`; the fixture `backlinks.json`).

Nothing new is stored. The cores read the links from the texts and keep them until the log changes (`workedOut`, `worked_out`), and the full-text index (ADR 0048) names the Notes and Sections whose text ever held the scheme, so a change reads only those and what they were merged into, never every Note; where the index cannot answer, every Note is read. A page asks for one Note's at a time, once per read and not per frame (#339).

It is the part at a Note's end, after its Sections, in the detailed view, the Note editor (the phone), the Note dialog and a Note's own window, its reading view too (the desktop): "Linked from", then one row per Note by its first line with words as it reads, its capture time where it has none. A tap or click opens it as its app link does. The compact view shows none, and a Note none links to shows nothing. Only Notes this reader may see are listed: what a store reads leaves out a Shared Area's Notes for who is no member of it (ADR 0013), and the phone leaves out the Notes of a locked Area while it is closed (Area Locks); the desktop has none. A card in the trash shows none. The AI port lists none (ADR 0026, ADR 0049): `read_note` is as it was.

## Considered

- Storing the links beside the text, or an index table of them (rejected, #345: nothing new is stored, and they would have to follow every edit, Merge, trash and role, while read from the texts they never go stale).
- `file://` links to a path (rejected: a path differs between Devices, the phone's shared folder has no path at all, and an absolute path could name anything on the disk).
- File links with no question (rejected, Q598: a link may come from another person, and opening a file starts another app).
- Typing ids into the Link dialog (rejected, Q596: nobody knows an id).
- Links in a Checklist's Lines (rejected for now: Lines are plain text on both apps; their editing and drawing would change everywhere).
- App links in the HTML for other apps (rejected: see above).

## Consequences

`Markdown.mayOpen` allows the app links `AppLink.parse` reads, so they draw and open as links on cards, in the read views and in the editors; the notifications' and the iPhone widget's own addresses of the scheme stay closed. The fixture `markdown.json` holds the parsed links, `mayOpen` and the labelled link edit. A Device of 0.4.x shows an app link as its words, which do not open.
