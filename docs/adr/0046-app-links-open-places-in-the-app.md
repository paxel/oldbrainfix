# App links open places in the app

A Note can link to another Note, an Area or Topic and the app's own pages, and the help Notes link each Quick start step to its place (#261, Q595–Q599). Both cores read and write these links the same way (`app_link.dart`, `app_link.rs`); the apps open them.

## An ordinary Markdown link with the app's scheme

An app link is a link of a formatted text (ADR 0033) whose destination has the scheme `oldbrainfix://`, in any case of ASCII letters, then one of these, written exactly so:

- `note/<id>` a Note, `area/<id>` an Area or a Topic, by the id it has on every Device;
- `agenda`, `unsorted`, `routines` (All Routines and Reminders), `profile`, `frogs`, `session/new` (New Session);
- `settings`, `settings/<section>` one of the Settings' sections (`look`, `units`, `agenda`, `alarms`, `nudges`, `capture`, `quick-note`, `calendar`, `privacy`, `typing`, `history`, `import`, `features`, `computer`), `settings/backups` and `settings/sync`;
- `guide` the user guide's start page, `guide/<page>` one of its pages and `guide/<page>#<box>` one of its boxes, the names lower-case letters, digits and `-`;
- `file/<path>` a file of the shared folder by its path inside it.

An id and a file's names are %-encoded but for ASCII letters, digits, `-`, `.`, `_`, `~`, `@` and, in an id, `:`, upper-case hex, so `[Dentist](oldbrainfix://note/note:3f2a)`. No query, no other `#`, no empty or extra part; anything else of the scheme is no app link. Nothing new is stored: the link is part of the text, so it syncs, is shared and goes into Packs as the text does, and the AI port reads it as written. A plain Markdown app shows its label.

## Opening

A link opens without asking. The Note opens in its editor (the phone) or the Note dialog (the desktop), an Area or Topic in the browser, the Agenda as the first page, the pages as they open from their menus, the Settings scrolled to the section where the app has it and at their top where it has not, and a guide page in the browser in the app's language, English where the site has none. A target the reader cannot see says "Not available here": a Note or Area this Device never got, in the trash or merged, a Note of a Shared Area the reader is no member of. A Note in a closed locked Area asks for the PIN first, as opening its Area does; the desktop has no Area Locks. New Session in Simple says it too, as only Advanced makes Sessions (ADR 0041).

## Files: only in the shared folder, after one question

A file link names a path inside the shared folder: on the phone the folder picked for sync (its document tree, walked name by name), on the desktop the synced folder (the path joined and resolved, symbolic links followed, and refused unless it still lies inside). The cores refuse empty names, `.`, `..`, `\`, `:` and control characters, so no link names a place outside the folder or a drive. Before it opens, one question names the file and the app the system opens it with ("Open "Scans/tax.pdf" from the shared folder with Document Viewer?"), "the app the system picks for it" where the system does not tell (a chooser on Android, macOS, Windows, the Flatpak, the iPhone). Nothing is imported, copied or shown in the app. A file the folder does not hold, or no shared folder, says "Not available here". On the iPhone the folder is the one picked in the Files app, kept as a bookmark; the file is found name by name inside it, refused once symbolic links lead out, and handed to the apps that take it through the system's Open In menu, which names no app, so the question says "the app the system picks for it"; a file of iCloud Drive not downloaded yet is not available until it is.

## Picking, not typing

The format bar's "Link to…" opens a picker of the app's pages, the Areas with their Topics and the Notes (by their first line as it reads), with a search over all of them; a Note of a closed locked Area is not offered. The one picked goes on the selection or the word at the cursor, or, without either, in at the cursor with its name as the label (`Markdown.link` with `label`, `markdown::link_labelled`). So nobody types an id. Simple has no format bar (ADR 0041), so no "Link to…" either; links already written open there too.

## HTML and other apps

The HTML handed to another app (#129) writes an app link as its label alone: another app cannot open it, and the system might hand the scheme to this app with nothing to say what to do.

## The help Notes

From 0.5.0 the Quick start and What the icons mean are formatted texts (ADR 0042, amended here): the Quick start's Description first lists, numbered as the Lines are, a link to each step's place (the Agenda, Unsorted, All Routines and Reminders twice, "Here, in this Checklist." for ticking a Line, the Agenda, Backups, Sync), then the desktop's way; What the icons mean ends with a link to the guide's start page. A Checklist's Lines are plain, so the links go into the Description, not into the Lines. Both apps write these texts from the same strings through `Markdown.link`, so the bytes are the same; the fixture `help` holds a link. Help Notes made before stay as they are (ADR 0042).

## Considered

- `file://` links to a path (rejected: a path differs between Devices, the phone's shared folder has no path at all, and an absolute path could name anything on the disk).
- File links with no question (rejected, Q598: a link may come from another person, and opening a file starts another app).
- Typing ids into the Link dialog (rejected, Q596: nobody knows an id).
- Links in a Checklist's Lines (rejected for now: Lines are plain text on both apps; their editing and drawing would change everywhere).
- App links in the HTML for other apps (rejected: see above).

## Consequences

`Markdown.mayOpen` allows the app links `AppLink.parse` reads, so they draw and open as links on cards, in the read views and in the editors; the notifications' and the iPhone widget's own addresses of the scheme stay closed. The fixture `markdown.json` holds the parsed links, `mayOpen` and the labelled link edit. A Device of 0.4.x shows an app link as its words, which do not open.
