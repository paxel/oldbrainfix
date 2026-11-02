# A Note is its kind: the kind is the page, the text its name

Lists and Tallies came into the capture-first model (ADR 0005) as "Make it a …": the Note's text stayed above as title and description, and the Lines or the Tally hung below it (ADR 0028, 0030). The user found every Note starting with a plain text block and "all the fancy stuff" under it; for a Shopping List the list should be the top of the whole thing (#188).

## One kind, and it is the page

A Note has exactly one kind: Plain Note, Checklist, Shopping List or Tally; the Session joins them with #191. A Frog is no kind: it stays a mark any Note can carry (ADR 0027). Nothing new is stored: the kind is read from what the Note holds already, a Tally while it has a field, else the list its `kind` names, else a plain Note, the Tally winning over a list as before (ADR 0030). Both cores read it the same way (`NoteKinds.of` in `note_kinds.dart`, `NoteKind::of` in `note_kinds.rs`); a Note whose Routine is a Session reads by its own fields until #191 gives the Session its place on the Note.

## The text is the name

For every kind but the Plain Note, the text shrinks to the name: its first line, up to the first line break of any kind, is the title, shown as a heading at the top of the page and of the full card. The rest of the text, as stored, is the description, shown small right under the name, folded to one line until a tap unfolds it. Then comes the kind's content (the Lines, the Tally), then the Note's settings as badges, its photos and recordings, and the Amends below as the dated diary part.

This is display only. No Note is rewritten to show this way, and the stored text keeps its format (ADR 0018, 0033). In the editor the name is a one-line field and the description its own field under it; saving writes them back as one text, the name alone while the description is empty, else the name, a line break and the description. A Note opened and left with neither changed writes nothing. Both cores split and join the same way (`NoteKinds.name`, `description`, `join`).

A list made from a Note keeps only its first line as text (ADR 0030), so it starts with no description; one shows once the user types it, or for a Tally, which keeps its whole text.

## The kind changes on its badge

The kind badge of the Note editor (#160) is on every Note now, a plain one too, and its dialog offers the kinds this Device may change to: from a Plain Note a Checklist, a Shopping List or a Tally; from a Checklist a Plain Note or a Shopping List; from a Shopping List a Plain Note or a Checklist. A Tally stays a Tally, since its fields are never removed (ADR 0028); its badge only shows its kind. The editor's own "Make it a Checklist" and "Make it a Shopping List" buttons go; the badge replaces them.

Going back to a Plain Note, and making a list, stay as they are (ADR 0030). Switching between the two lists is new: `ShoppingLists.switchList` (`switch_list`) writes the kind of the other list with a new making and, as Lines of that making, the Lines that show, in the order they show, all at one moment: a checked Line, or a struck one, comes over checked; every Line comes over at the top, with the reappear time a new Line gets on a Shopping List. A Checklist's Lines carry their own checks, not those of an Occurrence of its Routine. A Line gone Once stays behind. The text is not touched, so its description stays. What belonged to the old making, its Lines and their checks, stays in the log and shows no more, as when a list goes back to a Plain Note. It is allowed where making the list would be: on a Note in the lists that this Device may change, by its author or a Manager (ADR 0013).

## The AI port

`read_note` reports the name and the description beside the kind, null for a Plain Note. `switch_list` switches a list as a Pending Change (ADR 0026); it refuses a list with a checked or struck Line, as that Line would be checked again in the new making, and an AI Task checks nothing.

## Considered Options

- The kind as a field of its own (rejected: the Note already says it, by its fields and its `kind`, and a second place could disagree after a sync).
- Rewriting each Note into a name and a description field (rejected: every Note of every Device would be written again, and an older version would read a text it does not know).
- Switching lists through a Plain Note, back and made again (rejected: the text would be written twice, the description's lines would become Lines, and both writes could conflict).
- Carrying the Lines unchecked into the new list (rejected: what the user had checked would show open again; the round through a Plain Note already kept the "✓ ").
- The badges right under the name, above the kind's content (rejected: the list or the Tally is the page, so it comes first; the badges are its settings).

## Consequences

- A Plain Note shows as before.
- A Note whose kind changes keeps its text, so the name and the description stay.
- The Lines of every making stay in the log; a switch back and forth leaves Lines of each making.
- A Sent Item of a list switched here keeps the receiver's kind on the next update, as for a list changed by hand (ADR 0030).

## Changes to earlier decisions

- ADR 0005: a Note no longer starts with its text whatever it is; for a kind, the text is its name and description, and the kind's content comes first.
- ADR 0028: a Tally's text shows as its name and description, above its content.
- ADR 0030: a list's text "stays above as title and description" now as its name, heading the page, and its description folded under it; a Checklist and a Shopping List switch into each other with their Lines.
- ADR 0026: the AI port reads the name and description and switches a list (`switch_list`).
