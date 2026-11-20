# Spotlights explain the screens, and replace the first Help Notes

Amends ADR 0042.

People did not get along with the app (complaint 3 of 2026-11-20): the Quick start and "What the icons mean" sat as Notes in their own Area, away from the screens they were about. The user chose (obf-1043, #353, C3a to C3c) **Spotlights**, as cat-a-log has them: the page dims, one element is cut out of the dim, a card beside it says one short sentence, Next shows the screen's next tip, Skip ends the screen's tour.

## What a tour does

- Each screen has a manifest: its tips in order, each an id and its words. A ticket that adds something to a screen adds its tip there.
- On a screen's first frame, once it is the screen on top (no dialog, menu or other page over it, its page transition settled), the tips of the manifest not yet shown on this Device and whose element is on the screen now are shown one after another. A tip whose element is not there (Advanced only in Simple, Linked from without a link) stays due for a later visit. A tip of something inside a menu or dialog (Quiet today, Remind me…, the code language) points at what opens it. A Note in its own desktop window shows no tour; the Note dialog does.
- A tip is marked shown before it shows, so a tour broken off by anything never comes again.
- Skip, a tap or click outside the card, Esc and Back end the screen's tour and mark its other due tips shown. Nothing traps input: Skip is always on the card.
- The card stays inside the window, beside the element (on the side with room, over it only where a tall element leaves none), at most 400 wide, one sentence of at most 15 words, in English or German as the app.

So a person sees every screen explained on the first visit, and from then on, on each screen, only what is new on it, once (C3a (a)).

## Kept per Device, as local settings

The marks are local settings of the Device (ADR 0008), never synced and not settings of the person (ADR 0040): `spot:<screen>`, the ids of that screen's tips shown, comma-separated. A person with two Devices sees each tour once on each, which the user chose (C3c (a)): each app's screens differ, and a tip seen on the phone does not explain the desktop. "Show tips again" in the Settings of each app clears the marks of that Device, and every screen explains itself again on its next visit.

## An updated Device sees only what is new

A Device that ran an earlier version knows its screens. On the first start with Spotlights (`spotStarted` not yet set) a Device whose first start is not due marks every tip of a fixed list as shown: the tips of what existed in 0.6.1. Then it sets `spotStarted`, so this happens once, and Show tips again keeps it. The list is written out and frozen, never "the manifest now": a Device going from 0.6.1 straight to 0.8.0 still sees the tips of 0.7.0 and 0.8.0. A Device whose first start is due (a new person, a Device added, a Backup restored) gets every tour.

## The first Help Notes are no longer made

The Quick start and What the icons mean are no longer made, on a fresh install or on an update, by either core (C3b (a)). Making them only for a fresh start would not do: a person who starts on 0.7.0 never had them made, and the next update would make them as "never made". What's new in X.Y.Z stays as ADR 0042 has it, and so do its Area "Old Brain Fix", its ids and the made-once check. A Quick start or What the icons mean already made stays as the user has it, with its ids, its Agenda row and its links; a person who updates from 0.3.1 or earlier, the only one who never had them, does not get them now.

## Tests see none

As with the help Notes, the tours run only when the app is told to: by `main.dart` on the phone, by the desktop binary. Widget tests, kittests and the screenshot generators start without them, so no test waits behind a tip; the generators also mark every tip shown, so no guide frame shows one. The tour's own tests switch it on.

## Considered

- One "seen" mark per screen and version, as cat-a-log first had (rejected: a feature added later in the same version would never get its tip).
- Marks as a setting of the person, synced (rejected: the user chose per Device, C3c (a)).
- Keeping the Quick start for new people beside the Spotlights (rejected: C3b (a); two ways of explaining the same thing).
- Seeding from "every tip in the manifest" on an update (rejected: a Device skipping a version would miss that version's tips).

## Consequences

Every screen change has a tip to write or check. The Quick start's and the icons' strings, pictures and makers in the apps are left without a caller until a later cleanup removes them.
