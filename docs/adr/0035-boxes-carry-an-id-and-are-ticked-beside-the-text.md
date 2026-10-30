# Boxes carry an id in the text and are ticked beside it

Supersedes ADR 0031. A ☐, ✅ or ❌ in a Note's or a Section's text is a Box (grill Q402–Q409): it can be anywhere in the text, it is open, done or failed, and anyone with a role in its Area changes its state, in a Section's text too. The status marks of ADR 0031, characters at the start of a line changed by an edit of the text, never shipped; they are replaced. Both cores find, make, show and tick Boxes the same way (`text_boxes.dart`/`boxes.dart`, `text_boxes.rs`/`boxes.rs`).

## A Box in the text

A Box is its glyph, ☐ (U+2610) open, ✅ (U+2705) done or ❌ (U+274C) failed, one variation selector after it (U+FE0E or U+FE0F) belonging to it, and then its id: four tag characters, each a digit or a small letter (U+E0030–U+E0039, U+E0061–U+E007A, the tag forms of `0-9` and `a-z`), and a cancel tag (U+E007F). The id reads as those four ASCII characters, such as `a7k2`. The glyph is the state the Box starts in. A zero-width joiner after the glyph, or after its selector, makes it part of another emoji, which is no Box. An id that comes up a second time in a text makes no second Box: the second glyph shows as it is and counts for nothing until the text is saved again. Anything else after a glyph, an id of three or five characters, one without its cancel tag, a capital tag letter, is no id; the glyph is bare.

Tag characters are invisible, are never typed, and mean nothing in Markdown, so an id cannot collide with what a person writes, needs no escaping in plain or formatted text, and is not seen when the text is copied out of the field into another app. They belong to the glyph's grapheme cluster, so on the phone the cursor steps over a Box and its id as one character and a backspace takes both. The desktop's text renderer (egui) draws tag characters as replacement glyphs; its editor hides the id ranges itself, which the API gives it.

## Making Boxes when a text is saved

Whenever the text of a Note is captured or edited, and when a Section is added by Amend, every bare glyph that is no part of another emoji becomes a Box: an id is put right after it, after its selector. A Box whose id came up before in the text gets a new id, so a copied Box is a Box of its own. In a formatted text a glyph in inline code or in a link's address stays as it is; one in a link's label, a quote, a list item or a table's cell becomes a Box. An edit that changes nothing of this writes the text as it was. A Transcript or a Photo Text is never changed; taken over into the Note's text it is part of an edit, so its glyphs become Boxes there. A text accepted from a Sent Item, picked in a Conflict, or written back from a Checklist or Shopping List is written as it comes; its bare glyphs become Boxes at its next edit.

An id is drawn from the store's maker of entity ids: the first four bytes of the SHA-256 of a fresh entity id, each taken modulo the 36 characters, drawn again while the text holds it already. Ids are random on a Device and counted in the fixtures, the same in both cores.

## Ticks beside the text

Changing the state of a Box writes a tick: the field `box@<id>` on the entity whose text holds the Box, the Note, the Section, or the Note merged into another, whose text shows as a Section, with the state's name, `open`, `done` or `failed`. The moment of a tick is its entry's date. The latest tick of a Box by latest wins (date, recording time, Device, dseq) is its state; without a tick its glyph is. Every tick stays in the log, so a Box has its history: each tick that counts with its state, its moment and the Device that wrote it. A tick to the state a Box shows already writes nothing. A tick is read only in the shape the apps write: an id of four digits or small letters and one of the three names; anything else is skipped. Ticks are no Conflict unit.

A tick for an id the text no longer holds counts for nothing and stays in the log; when an edit, a Conflict pick or a restore brings the id back, its ticks count again. The glyph in the text stays the starting state: an author who types over a glyph changes the start, and a tick written later still wins.

## Who may tick

A tick lies on the Note or Section, so it is sealed under its Area's key, goes with the Note when it is filed into a Shared Area, with an offer from afar, in Bundles and in Backups. In a Shared Area anyone with a role ticks any Box of any Note and Section; the judge (ADR 0013) lets a Member write `box@…` on another person's Note, merged Note or Section. Adding, moving and deleting Boxes stays an edit of the text, the author's and the Managers'. Anywhere else only ticks the user's own Devices wrote count, as for a Session's picks (ADR 0029), so a tick another person's modified app slips into the log changes nothing.

Who ticked a Box last is read through the Device of its tick, as the author of a Note is (`tickedBy`/`ticked_by`): the user's own, another person as their Profile shows in the Shared Area, or nobody known. A blocked person's ticks are kept in their exact shape as their check-offs are (ADR 0021), and show without their name or picture.

## Reading

A Note and a Section carry their Boxes as they show now (`boxes`): each with its id, its state, its starting state, its glyph's range in the shown text, where the whole Box and its id lie in the stored text, and its latest tick. The shown text (`shownText`/`shown_text()`) has each Box in the bare glyph of its state and no id; the Boxes' ranges are counted in it, the formatted view parses it, and it is what is copied, shared to another app and saved as Markdown or HTML. The visible text, for search, previews, alarms and every plain projection, is made from it, so search never sees an id. The stored text with its ids is what the editor edits; after a save the editor takes the stored text back, so a glyph saved once keeps its id. A text in a format not known here has no Boxes and shows as it is.

## Sent Items

A Sent Item carries each text with every Box in the bare glyph of its state now, its id kept. The receiver's Boxes start from the sender's states, and the same ids take the receiver's own ticks. A tick by the sender changes the text the Sent Item carries, so an update replaces the receiver's text as any text change does (ADR 0015, ADR 0018), and the receiver's ticks, by the same ids, still win over the new glyphs. Bundles and Backups carry the ticks as entries.

## Considered Options

- A visible id in braces, such as `☐{a7}` (rejected: a person could type `✅{done}`, which would read as a Box and lose the word; the id would show in every text copied out of the field and on a Device of 0.1.x).
- The marks at the start of a line changed by an edit of the text, ADR 0031 (rejected: two people ticking different lines apart make a Conflict, a Member cannot tick another person's Note, and a Section's Boxes could not be ticked).
- Boxes found by their place in the text, the n-th glyph (rejected: a Box put in above would move every tick to the Box after it).
- A tick as an entity of its own, or one entity of ticks per Note (rejected: a field on the text's entity keeps each tick as its own signed entry, is sealed and filed with it, and needs no `$type` a Member would have to write).
- The ticks of a Section's Boxes on its Note (rejected: a merged Note's text shows under its target and travels with its own entity; the text's entity holds its own ticks either way).
- The tick's moment in its value (rejected: as for a Shopping List's check, the entry's date is the same everywhere).
- Changing the glyph in the text when a tick is written (rejected: it would be an edit of the text, the author's alone, and a Conflict with every concurrent edit).
- Converting a text accepted from a Sent Item (rejected: a sender's Boxes arrive with their ids; bare glyphs from a Device of 0.1.x wait for the next edit, so an update writing the same text again draws no ids).
- A Sent Item carrying the starting glyphs and the ticks beside them (rejected: the file format would grow a field a version without Boxes refuses; the glyph of the state now reads the same everywhere).

## Consequences

- No migration: no text with a Box or a status mark was ever shipped.
- A text with Boxes on a Device of 0.1.x shows the glyphs and, on the desktop, replacement glyphs for the ids; its edits there keep the ids.
- The API for the line-start marks (`TextMarks`, `Markdown.marks`/`swapMark`, `text_marks`, `markdown::marks`/`swap_mark`) stays until the apps have switched, then it is removed.
- The fixtures `text_boxes.json` and the `boxes` scenario with its `boxes.json` compare both cores.

## Changes to earlier decisions

- ADR 0031: superseded.
- ADR 0013: a Member writes the ticks of any Note's or Section's Boxes.
- ADR 0021: a blocked person's ticks of Boxes are kept, in their exact shape.
- ADR 0018, ADR 0033: saving a Note's or Section's text gives its bare glyphs ids; the visible text has no ids.
- ADR 0015: a Sent Item's texts carry each Box in its state now, its id kept.
