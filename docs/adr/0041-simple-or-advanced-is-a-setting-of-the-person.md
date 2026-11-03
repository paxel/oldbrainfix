# Simple or Advanced is a setting of the person; never chosen reads as Advanced

Tallies, Sessions and similar features confuse people who do not need them. The user chose (Q522 (a)) one switch, Simple or Advanced, asked at the first start with Simple picked, changeable in Settings any time, per person and the same on all of their own Devices (#197). Both cores read and write it the same way (`person_settings.dart`, `person_settings.rs`).

## It lies in the log like the other setting of the person

The switch is the field `features` of the entity `settings:person` (ADR 0040), so it travels to the user's own Devices only, and only what they wrote counts. It is `simple` or `advanced` once chosen. Choosing writes the value when the stored one differs, so Advanced picked on a fresh first start is written, not left absent; the same value again writes nothing. It carries no version.

## Never chosen reads as Advanced

Only `simple` hides anything. Absent, null and any value no app writes read as Advanced. Everyone who updates from 0.3.1 has no stored choice, and most of them have Notes; reading the absent value as Advanced keeps every feature they use where it was, without asking them and without writing a choice they did not make. A Device that joins the user's others or restores a Backup takes the choice that came with them, or reads Advanced where none came.

## The first start asks it on the Starter Set

Both apps already have a first start: its first screen, then for someone new the Starter Set. The question is on the Starter Set page, above the Areas, as two radio buttons with Simple picked, and Start keeps it together with the Areas. A Device of one's own being added and a restore do not ask. So no later start asks, and the tests and the screenshot generators, which start past the first start, never meet it. The generators choose Advanced where they show advanced features, as the last step of their seeding, so the ids and moments of what they seed before stay as they were.

## What Simple hides

Simple offers making Notes, Checklists, Shopping Lists, Reminders and Routines. In Simple the apps leave out:

- Tally, Session, Frog and Topic from "+" in the browser, and New Topic from an Area's menu on the desktop;
- Frog and Session from the spawn menus (a card's, the Note editor's and dialog's "+", the quick note's, the desktop's Note menu) and New Session from All Routines and Reminders;
- Tally and Session from the kind badge, and "Make it a Tally" from the menus and the quick note;
- the format bar (☐ and the formats) over the Note's text, its name and description, a Section and Add to Note, and in the quick note, with its switch in Settings ▸ Quick note; a field's own buttons, such as a Line's in and out, stay. The quick note in Simple is the one with that switch off, a plain field; a table's cells keep their bar, as a table only comes from a formatted text that exists; on the desktop Ctrl+B and Ctrl+I still format, as typed Markdown does;
- the role choice when sharing an Area, which then invites as a Member, and Make Manager and Make Member in People.

The Starter Set still offers its Topics in Simple: they are ticked on the same page, before Start, so they are planted, not made later.

Everything that exists stays visible and usable: a Tally takes entries and fields, a Session its pool, its picks and a new Tally from its pool, a Frog is picked and eaten, a Topic is opened and filed into, a formatted text shows formatted, typed and pasted Markdown still formats it and a table still opens, a person's role still shows and Remove stays.

## The AI port

`read_settings` reports the choice as `features`, `simple` or `advanced`. The AI port is not changed by it: every set-up tool, Tallies, Sessions, Frogs and Topics included, stays available in Simple, since the person asked the AI for it and accepts each change as a Pending Change. The choice is only read there, as every setting is.

## Considered

- Asking on the first Agenda open after an update (rejected: it would surprise people who already use the features, and every test that opens the Agenda would meet it).
- Counting a Device without Notes as Simple when nothing is stored (rejected: it would flip to Advanced with the first Note; absent reading as Advanced never changes by itself).
- A per-Device setting (rejected: Q522 (a), per person).
- Hiding Advanced data in Simple (rejected: Q522 (a), only making new ones is hidden).

## Consequences

Two of one's own Devices that choose apart before they meet end on the one chosen later, as for any field. The cross-core fixture `features` writes the same log from both cores, and `features.json` holds what each length of it reads as.
