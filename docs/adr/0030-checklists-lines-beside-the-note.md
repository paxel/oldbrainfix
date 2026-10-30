# Checklists and Shopping Lists: the kind on the Note, Lines beside it, checks as entries of their own

A Checklist (#120) is a Note whose Lines are checked off: its text stays above as title and description, each Line has a checkbox, and a Line can have a group of Lines under it. Both cores read and write it the same way (`checklists.dart`, `checklists.rs`). The Shopping List (#122) reuses the same Lines with a reappear time and no indents (`shopping_lists.dart`, `shopping_lists.rs`; see Shopping Lists below).

## The kind lies on the Note

Which list a Note is lies on the Note as `kind` = `checklist:<making id>`, a fresh id each time it is made one; null, or absent, for a plain Note. Lying on the Note, the kind is changed as the Note is: by its author, or by a Manager of its Area (ADR 0013). Making a Checklist writes the kind, the Note's first line as its text and, as a Tally does, `spawned`, which takes it out of Unsorted. A value that is not `<list>:<id>` with one colon names no list. A Note is at most one of a Tally and a Checklist: neither is made from the other, and a Note that holds both after two Devices made one each apart shows as the Tally.

## Lines are entities beside it

A Line is the entity `line:<id>` with `$type` = `line`, `note`, `made` (the making it belongs to), `text` (plain, as a Note's text is stored), `order`, `indent` (`true` while it is under the Line above), `checked` (the instant it was checked, null when unchecked) and `trashed` (when deleted, null when restored). Only the Lines of the making the kind names show, so turning a Checklist back into a plain Note writes the Note alone, its text and a null kind, and making it again starts with fresh Lines; no Line of an earlier making comes back, nor one a Member added meanwhile.

The Lines show by `order`, then by id. An order is a key of base-62 digits in ASCII order that does not end in `0`; a key fits between any two, so adding or moving a Line writes that Line alone, never its neighbours. A key at the end grows its first digit that can grow, so keys stay short. Keys of any other form sort after the valid ones.

The groups follow from that order: a Line marked `indent` belongs to the group of the nearest Line above it that is not marked, so there are two levels at most whatever arrives, and a first Line marked shows at the top. A Line moved before a Line of a group joins that group; a Line with a group moves with it and never into another; deleting a parent brings its group up to the top.

## Checks are entries of their own

Every check and uncheck is a `checked` entry on the Line; the Note's text is never touched. Concurrent checks and unchecks go by latest wins, so the later one stays; no Conflict is raised on them. A Line with a group shows checked while every Line of its group is checked, whatever its own `checked` says, so a check and an uncheck that meet after a sync never leave a parent checked over an unchecked Line. Checking a parent writes a check on every Line of its group not yet checked and on the parent; checking the last Line of a group writes one on the parent too; unchecking a Line of a group, or the parent, does the same the other way. "Uncheck all" writes an uncheck on every Line checked.

A Line's text conflicts as a Note's text does: it is written under `text`, so concurrent renames carry their causal context and are a Conflict with `$type` `line`, picked by whoever may rename it (ADR 0011).

## Who may do what

A Line lies in the Area of its Note: it is sealed under that Area's key, goes with the Note when the Note is filed into a Shared Area (`withFiled`), with an offer from afar and in a Backup. Anyone with a role in the Area adds Lines and checks or unchecks any Line. Renaming, deleting, restoring, moving and indenting a Line stay with the author of its Note, the person who wrote the Line's first `$type` and the Managers; the kind stays with the Note's author and the Managers. The Line's own author is among them because the judge cannot tell the writes that add a Line from later changes to it, as for a Section.

A blocked person's checks and unchecks are kept, in their exact shape, as their other check-offs are (ADR 0021); their Lines, which carry text, are refused and deleted.

## Sent Items

A Sent Item of a Checklist carries its Lines in the order they show, each with the sender's id for it, its text and whether it is under the Line above and shows checked, under `lines` in the item's `note`. Its file says `format` 2, so a version that does not know Lines refuses it instead of keeping the title alone; so does a Shopping List's (see Shopping Lists below), and a Sent Item of any other Note stays in format 1 (ADR 0019). Accepting one makes the new Note a Checklist of those Lines, not spawned, so it stays in Unsorted, each Line knowing the sender's id under `received`. An update adds the sender's Lines not there yet at the end, renames those whose text the sender changed since, takes away those the sender took away and makes the Note a Checklist when the sender's became one; once the sender's Note is a plain Note again, it becomes one here too when no Line of the receiver's own is left. What the receiver checked, renamed, added or moved stays.

## With a Routine or Reminder

A Routine or Reminder spawned from a Checklist's Note (#121) ties the two together: each new Occurrence starts with every Line unchecked, and the Occurrence is Done once every Line is checked, by anyone, as a Session's is once every pick has an entry (ADR 0029). Nothing is written when an Occurrence begins: a check counts only for the Occurrence it was made for. It is the field `check@<occurrence key>@<line id>` on the Routine, keyed by the wall-clock Occurrence as `done@` is, its value when it was checked, null when it was unchecked again; latest wins on each field. A new Occurrence has no such field yet, so its Lines show unchecked on every Device and for everyone in a Shared Area alike. The Line's own `checked` stays what it was, and shows again while the Note has no Routine or Reminder out of the trash.

Lying on the Routine, the checks are sealed under its Area key and go wherever the Routine goes. Anyone with a role in the Area writes them, as anyone checks off; in a Shared Area the roles decide (ADR 0013), and anywhere else only checks the user's own Devices wrote count, as for a Session's picks, so a check another person's modified app slips into the log changes nothing. A check is read only in the shape the apps write: an Occurrence key that reads back the same and a Line id with neither `@` nor `,`. Checking a Line for an Occurrence fans out to its group and its parent as a check on the Line does, each Line that changes getting its own field; "Uncheck all" for an Occurrence writes null on each one checked for it.

An Occurrence is Done from its checks once every Line counted for it shows checked: the Lines of the Note in the making the latest check belongs to, not deleted, but those added after that check and not checked for it, so a Line added later never takes a Done away from an Occurrence already complete. `Routine.done` then holds it with the moment of that latest check, so the Agenda, the alarm plan, Missed, the Streak, Adherence and the History take it as any check-off. A Checklist without a Line is never Done this way. Unchecking a counted Line undoes a Done that came from the checks, and so does restoring a deleted one or checking again after a Line was added; a check-off by hand (`done@`) makes it Done as for any Routine and keeps its own moment. Turning the Checklist back into a plain Note, or making it again, leaves what was Done from earlier makings as it was; the new making starts with no Line checked.

The Occurrence the Checklist shows on its card and in its editor is the one it is on now: of all the Routines and Reminders of its Note out of the trash, the Occurrences on the latest day up to today that has any, the first of them not Done, else the last; before their first day, the first of them. A Routine twice a day thus moves on to its evening once the morning is Done, a weekly one stays on its day until the next comes, and from the Agenda each Occurrence shows its own.

## Shopping Lists

A Shopping List is a Note whose Lines come back a while after they are checked. Its kind is `shopping:<making id>`, made and turned back into a plain Note as a Checklist's is, with the same Lines: the same entity, sealed and filed with its Note, the same roles, Conflicts on their text and latest wins on their checks. The kind holds one list, so a Note is at most one of a Tally, a Checklist and a Shopping List; one that holds a Tally's fields too shows as the Tally. Its Lines have no indent: making one turns every line but the first that is not blank into a Line at the top, indented or not, one marked "✓ " struck. The Lines made at once are all written at one moment, so they show in the order of the text. Turning it back into a plain Note writes the Lines that show into its text, the struck ones marked "✓ ".

Each Line holds its reappear time under `reappear`: `once`, `1w`, `2w`, `3w` or `4w`. A new Line gets `1w`; a value not known here reads as `1w`, so no Line goes away by a value this version cannot read. It is set as a Line is renamed, by the author of the Note or of the Line and the Managers.

Checking a Line writes `checked` as on a Checklist. A Line Once is then gone: it shows nowhere and search does not find it, but it is not deleted, since a Member may check any Line and may not delete another person's. Any other Line is struck through until the start of its day back: the day of its check on the reading Device's wall clock, as many weeks on as its reappear time says, so the same weekday. Nothing is written when it comes back. It is open again from the start of that day on every Device whose wall clock shows that day, so two Devices in different time zones may differ for the hours between them, and agree again after. The moment of a check is its entry's `date`, the same on every Device and in both cores whatever its value says.

The order follows from what is stored and the wall clock alone. The open Lines come first, the one that came last at the top: by the start of its day back, by the moment its uncheck was written, or by when it was added (its `$type`). The struck Lines follow, the one checked last at the end. Lines of the same moment go by key, then by id, so the Lines made at once keep the order of the text. Lines are not moved by hand: a new Line comes in at the top as it is the latest, with a key after every other.

Adding a name the list has already adds no Line. Names are compared trimmed and in lower case, character by character as Dart's `trim` and `toLowerCase` make them. An open Line of that name stays as it is; a struck or gone one is unchecked, so it comes back now, at the top, and keeps its reappear time. Anyone with a role in the Area may, as an uncheck is a check. A deleted Line is not brought back by its name.

On the Agenda (#123) a Line "came back today" when its last check runs out at the start of today on the Device's wall clock: an open Line whose day back is today. A Line brought back by hand or by adding its name did not come back by itself and does not count. Each Device chooses how today's Agenda shows Shopping Lists, as a setting of its own (ADR 0008, `shoppingAgenda` = `lines`, `back`, `open` or `off`, `back` when unset or unknown): each Line that came back today on its own row, one row per list with any, one row per list with open Lines, or none. The lists show oldest capture first, those in the trash or merged not at all. Both cores compute the rows the same way (`ShoppingLists.agenda`). Nothing is written when a Line comes back, so nothing is written for the Agenda either.

A Routine or Reminder spawned from a Shopping List is a plain one: its Occurrences are Done only when checked off, and the Lines show their own checks on every day. Only the Lines of a making that the Note's kind named as a Checklist count for a Done from checks, so a `check@` field naming a Line of a Shopping List counts for nothing.

A Sent Item of a Shopping List carries its Lines that show, in format 2 as a Checklist's, under `lines` in the item's `note` with `list` = `shopping` beside them. Each Line has the sender's id for it, its text and its reappear time under `reappear`; it says neither `indent` nor `checked`, since every Line arrives open, and those gone Once stay with the sender. Which Lines show struck depends on the wall clock, so the sender reads its list on UTC for their order alone: both cores send the same Lines in the same order on every Device. A `list` naming anything else, or a `list` without `lines`, makes the file no Sent Item; a Line's reappear time that is missing or not known here reads as `1w`, as on a Line. Accepting one makes the new Note a Shopping List of those Lines, all written at one moment, so they show in the sender's order, each with its reappear time and the sender's id under `received`, not spawned, so it stays in Unsorted. An update follows the rules of a Checklist's: it adds the sender's Lines not there yet, as the latest, so at the top; renames those whose text the sender changed since; takes away those the sender took away, a Line gone Once here too; makes the Note a Shopping List when it is a plain Note here and the sender's became one; and makes it a plain Note again once the sender's is one and no Line of the receiver's own is left. A reappear time the receiver set, and every check, stays as the receiver has it. A list here of the other kind stays as it is: a receiver who made a received Checklist a Shopping List, or a received Shopping List a Checklist, keeps it, and an update changes none of its Lines, as for a Tally.

## Considered Options

- The Lines as fields on the Note, as a Tally's fields are (rejected: a Member may only write on their own Notes, so adding a Line to someone else's Checklist would need a field-level exception the judge cannot tell from a rename).
- Each check as an entity of its own (rejected: a field per Line keeps every check and uncheck as its own signed entry already, and latest wins is the rule the issue asks for).
- The parent's state as stored (rejected: a check and an uncheck written apart can leave the parent checked over an unchecked Line; derived, it never is).
- A parent pointer per Line (rejected: concurrent indents could make three levels or a loop; an indent mark in one order cannot).
- Whole-number places renumbered when no gap is left (rejected: a Member adding a Line could not renumber the Lines of others).
- Trashing the Lines when going back to a plain Note (rejected: a Note's author who is a Member may not delete another Member's Line, and a later making would bring old Lines back).
- An added `lines` key in Sent Items of format 1 (rejected: an older version would keep the title and drop the Lines unseen).
- Every Device writing an uncheck on each Line when a new Occurrence begins (rejected: Devices that meet later would each write them, an offline one too late, and the log would grow with every Occurrence).
- A check counting for the Occurrence whose time span holds the moment it was made (rejected: it needs the wall clock of the Device reading, as ADR 0029 found for entries, and checking a Line before the Occurrence's time would count for the one before).
- The Occurrence in the value of the Line's own `checked` (rejected: the check of the next Occurrence would overwrite it and take the Done of the last one away).
- The checks of an Occurrence on the Line (rejected: a Note can spawn several Routines, and a check belongs to one Occurrence of one of them, as a Session's marks lie on its Routine).
- Done from every Line the Checklist has now (rejected: adding a Line would undo the Done of every Occurrence before and break the Streak).
- A Shopping List's Lines as a kind of entity of their own (rejected: the Line already carries everything but the reappear time, and the judge, the Area, search, Bundles, Backups and blocking know it).
- Writing an entry when a struck Line comes back (rejected: every Device would write it, an offline one too late, as for the start of an Occurrence).
- Trashing a Line Once when it is checked (rejected: a Member who checks it may not delete another person's Line, and a name added again could not bring it back).
- Moving a struck Line to the end, or a Line that comes back to the top, with a new `order` (rejected: a Member may not write another person's `order`, and a Line coming back writes nothing).
- The check's moment from its value (rejected: a value no app writes could read differently in the two cores; the entry's date is the same everywhere).
- Names compared by Unicode's full case folding (rejected: both cores lower each character the way Dart's `toLowerCase` does, which keeps them equal without a table of their own).
- Checks per Occurrence for a Routine spawned from a Shopping List (rejected: the reappear time brings its Lines back already, and a weekly Routine would open every Line again at each Occurrence).
- A Sent Item of a Shopping List carrying its text alone (rejected: the receiver would get a list without its Lines; the Lines go along, all open, so the wall clock decides nothing but their order).
- A Shopping List's Lines in a Sent Item struck as the sender sees them (rejected: which are struck depends on the sender's wall clock and changes by itself; open on arrival, the receiver starts from the whole list).
- The sender's wall clock for the order of a Shopping List's Lines in a Sent Item (rejected: the two cores and every Device would need the same zone to write the same file; UTC is the same everywhere).
- An update changing the reappear times of the Lines the receiver has (rejected: the receiver may have set their own; a new Line comes with the sender's).

## Consequences

- The AI port has no tool for Checklists, so an AI Task cannot change them (ADR 0026).
- Search finds a Note by a Line of its Checklist not deleted.
- A deleted Line stays in the log and can be restored; it is never purged.
- The Shopping List adds its own list to the kind and a reappear time to the Line; checked Lines sort to the end and come back at read time, with no write.
- The checks for Occurrences travel as Routines do: Sent Items carry a Routine's schedule only, and the AI port writes none of them (`check@` is not among the Routine's fields it takes).

## Changes to earlier decisions

- ADR 0005: making a Note a Checklist takes it out of Unsorted, as spawning does.
- ADR 0011: a Line's text conflicts as a Note's text does.
- ADR 0005: a Routine or Reminder spawned from a Checklist is Done once every Line is checked for its Occurrence.
- ADR 0013: any role adds and checks Lines; the rest is the Note's author's, the Line's author's and the Managers'. A Member writes a Routine's `check@` fields, as `done@`.
- ADR 0019: a Sent Item carrying a Checklist is in format 2.
- ADR 0021: a blocked person's checks of Lines are kept, those on the Line and those on a Routine, each in its exact shape.
- ADR 0029: a Routine's Done comes from its checks as a Session's comes from its entries; one that is both is Done from either.
- ADR 0028: a Note that is a list is never a Tally too.
- ADR 0005: making a Note a Shopping List takes it out of Unsorted; a Routine or Reminder spawned from one is Done only when checked off.
- ADR 0013: any role adds, checks and brings back the Lines of a Shopping List; their reappear time is the Note's author's, the Line's author's and the Managers'.
- ADR 0019: a Sent Item of a Shopping List is in format 2, with its Lines and `list` = `shopping`.
