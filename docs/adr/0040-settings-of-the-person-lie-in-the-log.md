# Settings of the person lie in the log; Done can move a Note to the trash

Settings belong to a Device and never sync (ADR 0008). The first setting that belongs to the person instead came with #211. Notes with a one-time Reminder lay around once it was Done. The user chose (Q533 (a)) a setting, "Move a Note to the Trash when its one-time Reminder is done", off by default, per person and the same on all of their own Devices. Both cores read and write it the same way (`person_settings.dart`, `person_settings.rs`).

## The setting is kept as an Override is

The person's settings are the entity `settings:person` in the log, outside every Area, as an Override (ADR 0017), today's Frog pick (ADR 0027) and "Show on the Agenda" (ADR 0030) are. It is sealed under the personal key. It goes to the user's own Devices only, by sync, the shared folder, bundles and Backups, and never to another person, a Shared Area's segments or a Sent Item. Only what the user's own Devices wrote counts (this one, one paired as its own, or one whose key is bound to the own Profile). It is read from the log alone, the newest such entry winning, so an entry another person's modified app slips into the log changes nothing.

The field `trashDoneReminder` is `on` while the setting is on. Switching it off writes null. Any other value reads as off, so nothing a later version or a modified app writes moves a Note to the trash. The same value again writes nothing. It carries no version. A later setting of the person is a field of its own on the same entity.

## Done moves the Note with its Reminder to the trash

Done goes through one seam in each core (`Routines.checkOff`, `Routines::check_off`). That is the Agenda's box and its Missed rows, a card's box (#215), the Note editor's and dialog's Done, a notification's Done applied later (`applyDone`, `apply_done`) and so the phone's widget and full-screen alarm. The seam checks the Occurrence off. Then, with the setting on, it moves the Note to the trash with its Reminder, as "Done and move to trash" does (#215, `trashNote` with its Routines), when all of these hold:

- the Routine is a Reminder (it happens once);
- its Note, the one it shows under (the Note it was merged into, if so), is in the lists, not merged;
- the Note is a Plain Note (ADR 0038): never a Checklist, a Shopping List, a Tally or a Session;
- the Reminder is the Note's only Routine or Reminder out of the trash;
- this Device may change the Note (ADR 0013). A Member in a Shared Area checks off another person's Reminder, and their own setting moves nothing of that person's Note.

A Frog on the Note does not stop it; it goes to the trash with the Note, as with "Done and move to trash". `markDone` stays the raw write of the check-off. Done that arrives by sync from another Device trashes nothing here: the Device where Done was tapped wrote the trash, and that arrives with it.

## Unchecking does not bring the Note back

Once the Note is in the trash, its Reminder is too, so there is no row or box left to uncheck. Unchecking is no undo. The Note comes back with its Reminder from the trash for 30 days (`restoreNote`), still Done. Unchecking it there leaves the Note where it is.

## Considered

- A per-Device setting (rejected: Q533 (a), the user wants it the same on their own Devices).
- Trashing a Note of any kind (rejected: a list, a Tally and a Session keep living after their Reminder).
- Trashing in `markDone` itself (rejected: one seam that the UI calls keeps the raw write for imports, tests and the AI port's drafts).
- Trashing only the Notes the person wrote (rejected: #215's button moves any Note this Device may change, a Manager's too, and the setting follows it).

## Consequences

Two of one's own Devices that check off the same Reminder apart, before they meet, each move the Note to the trash. The second trash moment wins after a sync, as for any field.

The cross-core fixture `done-trash` writes the same log from both cores.

## Simple or Advanced (ADR 0041)

Amended by #197: the second setting of the person, Simple or Advanced, is the field `features` of the same entity, `simple` or `advanced`; never chosen and any other value read as Advanced (ADR 0041).

## The Areas in the person's own order (#280)

Amended by #280 (Q676 (a)): the user drags the Areas into an order of their own. It is the third setting of the person, so the other people of a Shared Area keep their own order. It is the field `areaOrder` of the same entity: a JSON list of Area ids, first to last, with no version. It is read from the log alone, by the user's own Devices' entries, the newest winning, as the other two settings are. An order another person's Device slips into the log changes nothing.

Reading it (`areasInOrder`, `areas_in_order`) lists the Areas named in the order first, as it has them. Then come the Areas missing from it, by name, as the Areas are listed without an order: a new Area, or a Shared Area joined later. Ids that are no Area here, Topics among them, are skipped, as are repeats after the first and anything that is not a string. A value that is not a JSON list reads as no order, so all Areas show by name, as before.

Moving an Area (`moveArea`, `move_area`) or setting the order (`setAreaOrder`, `set_area_order`) stores the whole order as shown. A later Area therefore goes to the end. Ids that are no Area here are dropped. An order equal to the one shown writes nothing, so nothing is stored while the user never moves an Area off its place by name. Moving an id that is no Area is refused. A starter Area shared under fresh ids (ADR 0013) keeps its place: sharing writes the order with the new id where the old one stood. `Places.areas`/`Places::areas` stay by name for everything that is not the Areas list: Packs, Imports, Backups and the transcript prompts.

The cross-core fixture `area-order` writes the same log from both cores and reads the same order after each step.
