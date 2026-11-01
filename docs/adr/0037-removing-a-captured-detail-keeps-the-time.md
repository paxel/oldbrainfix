# Removing a captured detail keeps the capture time

A Capture Context records what was true at Capture, and some of it is noted by chance: "during: Katzenklo" (cat litter) on a work Note stays for good (#162). Each detail can now be removed on its own: the place (the coordinates, and with them the Named Place they lie in), the calendar events running, the Bluetooth devices connected and the app it was shared in from. The capture time is no such detail and stays. This follows removing a recording while its Transcript stays (ADR 0020, 0025).

## Removing

Removing is an edit to the Note, or to a Section, which carries its own Capture Context: the detail's fields set to null, version 1 entries with no key of their own, signed, synced and won by latest wins like any other. The place is `location`, the calendar events `calendar`, the Bluetooth devices `bluetooth`, and the app `sourcePackage` with `sourceLabel` beside it, both in one batch, so no label is left over of an app that was removed. Null now means removed; absent still means not recorded. Both cores read a null as no detail, as they did before, so a removed detail no longer shows on a card, in the editor or anywhere else a Note is read through the core, and is not in what the AI port reads (`read_note`). Nothing searches or filters by a captured detail yet; whatever does later reads the same Capture Context.

The calendar events are removed together, as one detail, as are the Bluetooth devices: the badge's editor shows them as one line each. A Named Place itself is not touched: it stays for every other Note captured there, and the Note no longer lies in it because it has no coordinates.

Both cores offer it as `mayRemoveCaptured` / `removeCaptured` on Notes (`may_remove_captured` / `remove_captured` in Rust) with a `CapturedDetail` of `place`, `event`, `device` or `app`, for a Note, an Amend's Section or a merged Note shown as a Section. They refuse a detail that does not show, garbled or removed before, so a removal is never written twice.

## Roles

They also refuse it on a Note or Section this Device may not change (`mayChange`, ADR 0013): in no Area or one it manages, or as a Member only on Notes and Sections they made. The judge needs no new rule: a Member may write the fields of a Capture Context only on what they made, as any other field but a text their machine adds and a box's tick, so a Member's removal on another person's Note, written all the same, is ignored on every Device. A Manager removes a detail on anyone's Note, as they edit anyone's text.

## Where it goes

The removal is an entry, so LAN sessions, the shared folder and Bundles carry it to every Device that holds the Note, a Shared Area's people included, and a Backup holds it. A Sent Item never carries a Capture Context (ADR 0009), so the person receiving it gets none of the details either way.

## The AI port

Removing a detail is something the user does with their own Notes, so the AI port has a tool for it (`remove_captured`, ADR 0026), drafted as a Pending Change of the AI Task until the user accepts it. An AI Task may write the fields of a Capture Context only as null: it removes a detail, it never says where or with what something was captured. Its review shows such a removal among "anything else".

## Older versions

No version marker is needed (ADR 0034): an older version reads the null as every version reads it, as no detail, so the removal shows there too. It still lets a Member's removal on another person's Note be ignored, as it judges these fields the same way.

## Considered Options

- Removing a single calendar event or Bluetooth device out of a list (rejected: the editor shows each list as one detail, and a list written again would read as a new detail rather than a removal).
- An empty value instead of null (rejected: the AI port would still show the field, and an empty list would read as "nothing was running" rather than as removed).
- Leaving the app's label when its package is removed (rejected: the label names the app, and the AI port reads the raw fields).
- Letting the capture time go too (the time places the Note in the diary).

## Consequences

- A removed detail stays in the log, as an edited text does; a Backup made before the removal still holds it.
- A position or calendar lookup that arrives after a removal writes the detail again, as any later write wins. Capture attaches them moments after the Note is saved, and only then.
